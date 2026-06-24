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
    let mut resp = client()?.get(url).send().await?;
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
