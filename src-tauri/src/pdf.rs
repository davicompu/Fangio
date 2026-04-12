//! PDF text extraction and chunking.
//!
//! Extraction strategy (in order):
//! 1. `pdftotext` (poppler) — superior font/encoding support, no glyph warnings.
//!    Present on macOS via `brew install poppler`; common on Linux distributions.
//! 2. Pure-Rust `pdf-extract` fallback — works without system dependencies but
//!    struggles with non-standard font encodings (logs "unknown glyph name" for
//!    OpenType ligature glyphs such as uniFB01/uniFB02).
//!
//! PDFs are copied into `<workspace>/pdfs/` so projects remain portable.

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

const TARGET_CHUNK_CHARS: usize = 800;
const CHUNK_OVERLAP_CHARS: usize = 100;

pub struct ExtractedPdf {
    pub text: String,
    pub page_count: Option<usize>,
}

pub fn extract(path: &Path) -> Result<ExtractedPdf> {
    // Prefer pdftotext — handles all standard font encodings cleanly.
    if let Ok(text) = extract_via_pdftotext(path) {
        return Ok(ExtractedPdf {
            text: normalize_text(&text),
            page_count: None,
        });
    }
    // Fallback: pure-Rust pdf-extract (no system dependency required).
    extract_via_pdf_extract(path)
}

/// Try pdftotext (poppler). Returns Err if the binary is not found or fails.
fn extract_via_pdftotext(path: &Path) -> Result<String> {
    let binary = find_pdftotext().ok_or_else(|| anyhow!("pdftotext not found"))?;
    let output = std::process::Command::new(&binary)
        .args(["-enc", "UTF-8", "-nopgbrk"])
        .arg(path)
        .arg("-") // output to stdout
        .output()
        .context("failed to run pdftotext")?;
    if !output.status.success() {
        return Err(anyhow!(
            "pdftotext exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Locate the pdftotext binary in common install paths.
fn find_pdftotext() -> Option<PathBuf> {
    // Check PATH-resolved name first, then common Homebrew locations.
    let candidates = [
        "pdftotext",
        "/opt/homebrew/bin/pdftotext", // Apple Silicon Homebrew
        "/usr/local/bin/pdftotext",    // Intel Homebrew / Linux
        "/usr/bin/pdftotext",          // Debian/Ubuntu package
    ];
    for candidate in candidates {
        let p = PathBuf::from(candidate);
        // For bare names we rely on PATH; for absolute paths we check existence.
        if p.is_absolute() {
            if p.exists() {
                return Some(p);
            }
        } else {
            // Try spawning with just the name — if it fails the binary isn't in PATH.
            if std::process::Command::new(candidate)
                .arg("-v")
                .output()
                .is_ok()
            {
                return Some(p);
            }
        }
    }
    None
}

/// Fallback extractor using the pure-Rust `pdf-extract` crate.
fn extract_via_pdf_extract(path: &Path) -> Result<ExtractedPdf> {
    // Some PDFs have non-standard encodings that cause pdf-extract to panic
    // (e.g. "missing unicode map and encoding"). Catch any panic and convert
    // it to an error so the caller can skip the file gracefully.
    let path_owned = path.to_path_buf();
    let result = std::panic::catch_unwind(move || pdf_extract::extract_text(&path_owned));
    let raw = match result {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => {
            return Err(anyhow::anyhow!("pdf-extract error: {e}"));
        }
        Err(_) => {
            return Err(anyhow::anyhow!(
                "pdf-extract panicked (likely missing unicode map or unsupported encoding)"
            ));
        }
    };
    Ok(ExtractedPdf {
        text: normalize_text(&raw),
        page_count: None,
    })
}

/// Normalize common PDF extraction artifacts so the text is clean for embedding.
///
/// Many academic PDFs (especially those using fonts like CharisSIL) encode
/// typographic ligatures as Unicode Letterlike Symbols (U+FB00–U+FB06) rather
/// than multi-character sequences. pdf-extract maps them correctly but the
/// resulting characters (`ﬁ`, `ﬂ`, `ﬀ`, etc.) look like gibberish to most
/// embedding models. We decompose them back to their component letters.
fn normalize_text(text: &str) -> String {
    // Pre-allocate with a small headroom since replacements are same length or shorter.
    let mut out = String::with_capacity(text.len() + 64);
    for ch in text.chars() {
        match ch {
            '\u{FB00}' => out.push_str("ff"),
            '\u{FB01}' => out.push_str("fi"),
            '\u{FB02}' => out.push_str("fl"),
            '\u{FB03}' => out.push_str("ffi"),
            '\u{FB04}' => out.push_str("ffl"),
            '\u{FB05}' => out.push_str("st"),
            '\u{FB06}' => out.push_str("st"),
            // Superscript digits sometimes appear in reference numbers
            '\u{2070}' => out.push('0'),
            '\u{00B9}' => out.push('1'),
            '\u{00B2}' => out.push('2'),
            '\u{00B3}' => out.push('3'),
            '\u{2074}' => out.push('4'),
            '\u{2075}' => out.push('5'),
            '\u{2076}' => out.push('6'),
            '\u{2077}' => out.push('7'),
            '\u{2078}' => out.push('8'),
            '\u{2079}' => out.push('9'),
            // Soft hyphen (invisible break hint) — drop it
            '\u{00AD}' => {}
            // Zero-width space / non-breaking space → regular space
            '\u{200B}' | '\u{FEFF}' => {}
            '\u{00A0}' => out.push(' '),
            _ => out.push(ch),
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub index: usize,
    pub text: String,
}

/// Sentence-aware chunking. Splits text on paragraph boundaries first, then
/// packs consecutive paragraphs until ~TARGET_CHUNK_CHARS, with light overlap
/// between consecutive chunks so retrieval doesn't miss boundary information.
pub fn chunk(text: &str) -> Vec<Chunk> {
    let cleaned = text
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    let paras: Vec<&str> = cleaned
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    let mut chunks = Vec::new();
    let mut buf = String::new();
    let mut idx = 0usize;
    for p in paras {
        if buf.is_empty() {
            buf.push_str(p);
        } else if buf.len() + 2 + p.len() <= TARGET_CHUNK_CHARS {
            buf.push_str("\n\n");
            buf.push_str(p);
        } else {
            chunks.push(Chunk {
                index: idx,
                text: buf.trim().to_string(),
            });
            idx += 1;
            // Carry the tail of the previous chunk for overlap
            let tail: String = buf
                .chars()
                .rev()
                .take(CHUNK_OVERLAP_CHARS)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            buf = if tail.is_empty() {
                String::new()
            } else {
                format!("{tail}\n\n{p}")
            };
        }
        // If a single paragraph blows past the target, hard-split it
        while buf.len() > TARGET_CHUNK_CHARS * 2 {
            let cut = buf
                .char_indices()
                .take_while(|(i, _)| *i < TARGET_CHUNK_CHARS)
                .last()
                .map(|(i, c)| i + c.len_utf8())  // advance past the full char, not just 1 byte
                .unwrap_or(buf.len())
                .min(buf.len());
            let head = buf[..cut].to_string();
            let rest = buf[cut..].to_string();
            chunks.push(Chunk {
                index: idx,
                text: head.trim().to_string(),
            });
            idx += 1;
            buf = rest;
        }
    }
    if !buf.trim().is_empty() {
        chunks.push(Chunk {
            index: idx,
            text: buf.trim().to_string(),
        });
    }
    chunks
}

/// Copy a PDF into the workspace's `pdfs/` directory and return the new path.
pub fn copy_into_workspace(workspace: &Path, source: &Path) -> Result<PathBuf> {
    let dir = workspace.join("pdfs");
    std::fs::create_dir_all(&dir)?;
    let filename = source
        .file_name()
        .ok_or_else(|| anyhow!("invalid pdf filename"))?;
    let dest = dir.join(filename);
    if !dest.exists() {
        std::fs::copy(source, &dest)?;
    }
    Ok(dest)
}
