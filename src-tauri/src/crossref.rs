//! Tiny client for the Crossref REST API. Used to fill in missing article
//! metadata from a DOI. Returns Option-wrapped fields so callers can pick
//! only the ones they need without overwriting user data.

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Subset of the Crossref `/works/{doi}` response we care about.
#[derive(Debug, Default, Clone, Serialize)]
pub struct CrossrefRecord {
    pub title: Option<String>,
    pub authors: Option<String>,
    pub year: Option<i32>,
    pub journal: Option<String>,
    pub abstract_text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope {
    message: ApiMessage,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ApiMessage {
    title: Vec<String>,
    author: Vec<ApiAuthor>,
    issued: Option<ApiDate>,
    #[serde(rename = "container-title")]
    container_title: Vec<String>,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ApiAuthor {
    given: Option<String>,
    family: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ApiDate {
    #[serde(rename = "date-parts")]
    date_parts: Vec<Vec<i32>>,
}

/// Strip JATS / pseudo-XML tags Crossref often wraps abstracts in
/// (`<jats:p>`, `<jats:sec>`, etc). Conservative — drops anything that
/// looks like a tag, leaves text content as-is.
fn strip_jats(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    // collapse repeated whitespace
    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.trim().to_string()
}

fn normalise_doi(raw: &str) -> String {
    let s = raw.trim();
    let s = s.strip_prefix("https://doi.org/").unwrap_or(s);
    let s = s.strip_prefix("http://doi.org/").unwrap_or(s);
    let s = s.strip_prefix("doi:").unwrap_or(s);
    s.trim().to_string()
}

pub async fn fetch(doi: &str) -> Result<CrossrefRecord> {
    let normalised = normalise_doi(doi);
    if normalised.is_empty() {
        return Err(anyhow!("empty DOI"));
    }
    // URL-encode the DOI path segment (DOIs may contain '/')
    let encoded: String = normalised
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' | '/' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect();
    let url = format!("https://api.crossref.org/works/{encoded}");

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        // Crossref asks API users to set a polite User-Agent identifying the app
        // and a contact. Use a clear app name; researchers can sometimes get
        // routed to the polite pool.
        .user_agent("Reeves/0.1 (https://reeves.app; mailto:hello@reeves.app)")
        .build()?;

    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("crossref returned {}", resp.status()));
    }
    let env: ApiEnvelope = resp.json().await?;
    let m = env.message;

    let title = m.title.into_iter().next().filter(|s| !s.trim().is_empty());

    let authors_str: Option<String> = if m.author.is_empty() {
        None
    } else {
        let parts: Vec<String> = m
            .author
            .iter()
            .filter_map(|a| {
                match (a.given.as_deref(), a.family.as_deref()) {
                    (Some(g), Some(f)) if !g.trim().is_empty() && !f.trim().is_empty() => {
                        Some(format!("{f}, {g}"))
                    }
                    (None, Some(f)) | (Some(""), Some(f)) if !f.trim().is_empty() => Some(f.to_string()),
                    _ => None,
                }
            })
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("; "))
        }
    };

    let year = m
        .issued
        .as_ref()
        .and_then(|d| d.date_parts.first())
        .and_then(|p| p.first())
        .copied();

    let journal = m
        .container_title
        .into_iter()
        .next()
        .filter(|s| !s.trim().is_empty());

    let abstract_text = m.abstract_text.map(|s| strip_jats(&s)).filter(|s| !s.is_empty());

    Ok(CrossrefRecord {
        title,
        authors: authors_str,
        year,
        journal,
        abstract_text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_jats_handles_simple_tags() {
        let s = "<jats:p>Hello <jats:italic>world</jats:italic>.</jats:p>";
        assert_eq!(strip_jats(s), "Hello world.");
    }

    #[test]
    fn strip_jats_collapses_whitespace() {
        let s = "<p>One.</p>\n\n  <p>Two.</p>";
        assert_eq!(strip_jats(s), "One. Two.");
    }

    #[test]
    fn normalise_doi_strips_common_prefixes() {
        assert_eq!(normalise_doi("https://doi.org/10.1/x"), "10.1/x");
        assert_eq!(normalise_doi("doi:10.1/x"), "10.1/x");
        assert_eq!(normalise_doi("  10.1/x  "), "10.1/x");
    }
}
