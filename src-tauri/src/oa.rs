//! Open-access PDF discovery via Unpaywall (https://unpaywall.org).
//!
//! Returns links ONLY to legally open-access full text — Unpaywall indexes
//! author/publisher OA copies, never paywalled PDFs. This is the one feature
//! besides Crossref enrichment that reaches the internet.

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
struct OaLocation {
    url_for_pdf: Option<String>,
}

#[derive(Deserialize)]
struct UnpaywallResp {
    best_oa_location: Option<OaLocation>,
    #[serde(default)]
    oa_locations: Vec<OaLocation>,
}

fn normalise_doi(raw: &str) -> String {
    let s = raw.trim();
    let s = s.strip_prefix("https://doi.org/").unwrap_or(s);
    let s = s.strip_prefix("http://doi.org/").unwrap_or(s);
    s.trim().to_string()
}

fn client() -> Result<Client> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Reeves/0.1 (https://reeves.app; mailto:hello@reeves.app)")
        .build()?)
}

/// True if the URL points at a non-public host (loopback / private / link-local
/// / unspecified). OA PDF URLs come from arbitrary mirrors chosen by the
/// Unpaywall response, so this blocks SSRF at internal services and cloud
/// metadata endpoints — both for the initial URL and every redirect hop.
fn host_is_blocked(url: &reqwest::Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(h)) => {
            let h = h.to_ascii_lowercase();
            h == "localhost" || h.ends_with(".localhost")
        }
        Some(url::Host::Ipv4(ip)) => {
            ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_unspecified() || ip.is_broadcast()
        }
        Some(url::Host::Ipv6(ip)) => {
            let seg0 = ip.segments()[0];
            ip.is_loopback()
                || ip.is_unspecified()
                || (seg0 & 0xfe00) == 0xfc00 // unique local fc00::/7
                || (seg0 & 0xffc0) == 0xfe80 // link-local fe80::/10
        }
        None => true,
    }
}

/// Download client that refuses to follow a redirect into a non-public host and
/// caps redirect depth.
fn download_client() -> Result<Client> {
    let policy = reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= 10 {
            attempt.error("too many redirects")
        } else if host_is_blocked(attempt.url()) {
            attempt.error("redirect to a non-public host blocked")
        } else {
            attempt.follow()
        }
    });
    Ok(Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Reeves/0.1 (https://reeves.app; mailto:hello@reeves.app)")
        .redirect(policy)
        .build()?)
}

/// A direct open-access PDF URL for the DOI, if one exists.
pub async fn find_pdf_url(doi: &str) -> Result<Option<String>> {
    let d = normalise_doi(doi);
    if d.is_empty() {
        return Ok(None);
    }
    let encoded: String = d
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' | '/' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect();
    let url = format!("https://api.unpaywall.org/v2/{encoded}?email=hello@reeves.app");
    let resp = client()?.get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let data: UnpaywallResp = resp.json().await?;
    if let Some(u) = data.best_oa_location.and_then(|l| l.url_for_pdf) {
        return Ok(Some(u));
    }
    for loc in data.oa_locations {
        if let Some(u) = loc.url_for_pdf {
            return Ok(Some(u));
        }
    }
    Ok(None)
}

/// Largest open-access PDF we will accept — guards against a hostile or buggy
/// mirror returning a multi-GB body and exhausting memory.
const MAX_PDF_BYTES: usize = 100 * 1024 * 1024; // 100 MB

/// Download a PDF, verifying the magic bytes so we never save an HTML error page.
/// Reads incrementally with a hard size cap so a malicious host can't OOM us.
pub async fn download(url: &str) -> Result<Vec<u8>> {
    let parsed = reqwest::Url::parse(url).map_err(|e| anyhow!("invalid PDF url: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(anyhow!("refusing to download from a non-http(s) url"));
    }
    if host_is_blocked(&parsed) {
        return Err(anyhow!("refusing to download from a non-public host"));
    }
    let mut resp = download_client()?.get(parsed).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("download returned {}", resp.status()));
    }
    // Reject early when the server advertises an oversized body.
    if let Some(len) = resp.content_length() {
        if len as usize > MAX_PDF_BYTES {
            return Err(anyhow!("PDF too large ({len} bytes)"));
        }
    }
    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await? {
        if bytes.len() + chunk.len() > MAX_PDF_BYTES {
            return Err(anyhow!("PDF exceeded the {MAX_PDF_BYTES}-byte limit"));
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.len() < 5 || &bytes[0..5] != b"%PDF-" {
        return Err(anyhow!("response was not a PDF"));
    }
    Ok(bytes)
}
