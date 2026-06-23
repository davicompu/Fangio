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

/// Download a PDF, verifying the magic bytes so we never save an HTML error page.
pub async fn download(url: &str) -> Result<Vec<u8>> {
    let resp = client()?.get(url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("download returned {}", resp.status()));
    }
    let bytes = resp.bytes().await?.to_vec();
    if bytes.len() < 5 || &bytes[0..5] != b"%PDF-" {
        return Err(anyhow!("response was not a PDF"));
    }
    Ok(bytes)
}
