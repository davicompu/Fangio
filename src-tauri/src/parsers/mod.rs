//! Bibliographic record parsers.
//!
//! Each format-specific module produces `ParsedRecord`s. The `parse_file`
//! function picks a parser by file extension or content sniffing.

pub mod bibtex;
pub mod ris;
pub mod scopus_csv;
pub mod wos_tab;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ParsedRecord {
    pub title: String,
    pub authors: Option<String>,
    pub year: Option<i32>,
    pub journal: Option<String>,
    pub abstract_text: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
    /// Tracks which raw fields the parser couldn't map (debug aid)
    #[serde(default)]
    pub extras: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FormatKind {
    Ris,
    BibTex,
    WosTab,
    ScopusCsv,
}

impl FormatKind {
    pub fn label(&self) -> &'static str {
        match self {
            FormatKind::Ris => "RIS",
            FormatKind::BibTex => "BibTeX",
            FormatKind::WosTab => "Web of Science (tab-delimited)",
            FormatKind::ScopusCsv => "Scopus CSV",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParseResult {
    pub format: FormatKind,
    pub records: Vec<ParsedRecord>,
    pub warnings: Vec<String>,
}

pub fn detect_format(path: &Path, content_head: &str) -> Result<FormatKind> {
    // 1. Extension hint
    if let Some(ext) = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
        match ext.as_str() {
            "ris" => return Ok(FormatKind::Ris),
            "bib" | "bibtex" => return Ok(FormatKind::BibTex),
            "txt" | "tsv" => {
                // WoS exports are .txt or .tsv
                if content_head.contains("PT\t") || content_head.starts_with("FN ") {
                    return Ok(FormatKind::WosTab);
                }
            }
            "csv" => return Ok(FormatKind::ScopusCsv),
            _ => {}
        }
    }
    // 2. Content sniffing
    let head = content_head.trim_start();
    if head.starts_with("TY  -") || head.starts_with("TY -") {
        return Ok(FormatKind::Ris);
    }
    if head.starts_with('@') {
        return Ok(FormatKind::BibTex);
    }
    if head.starts_with("FN ") || head.starts_with("PT\t") {
        return Ok(FormatKind::WosTab);
    }
    if head.contains("\"Authors\",\"Author full names\"")
        || head.starts_with("Authors,Title,Year")
    {
        return Ok(FormatKind::ScopusCsv);
    }
    Err(anyhow!(
        "could not determine bibliographic format for this file"
    ))
}

pub fn parse_file(path: &Path) -> Result<ParseResult> {
    let raw = std::fs::read(path)?;
    // Strip UTF-8 BOM if present
    let bytes: &[u8] = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &raw[3..]
    } else {
        &raw[..]
    };
    let content = String::from_utf8_lossy(bytes).into_owned();
    let head: String = content.chars().take(2048).collect();
    let format = detect_format(path, &head)?;
    let result = match format {
        FormatKind::Ris => ris::parse(&content),
        FormatKind::BibTex => bibtex::parse(&content),
        FormatKind::WosTab => wos_tab::parse(&content),
        FormatKind::ScopusCsv => scopus_csv::parse(&content),
    }?;
    Ok(ParseResult {
        format,
        records: result.0,
        warnings: result.1,
    })
}

/// Normalize a DOI for comparison: lowercase, strip prefixes, trim.
pub fn normalize_doi(doi: &str) -> Option<String> {
    let s = doi.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }
    let s = s
        .strip_prefix("https://doi.org/")
        .or_else(|| s.strip_prefix("http://doi.org/"))
        .or_else(|| s.strip_prefix("doi:"))
        .unwrap_or(&s)
        .trim()
        .to_string();
    if s.starts_with("10.") {
        Some(s)
    } else {
        None
    }
}
