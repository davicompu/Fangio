//! PDF text extraction and chunking.
//!
//! Local-only via the `pdf-extract` crate (pure Rust). PDFs are copied into
//! `<workspace>/pdfs/` so projects remain portable.

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

const TARGET_CHUNK_CHARS: usize = 800;
const CHUNK_OVERLAP_CHARS: usize = 100;

pub struct ExtractedPdf {
    pub text: String,
    pub page_count: Option<usize>,
}

pub fn extract(path: &Path) -> Result<ExtractedPdf> {
    let text = pdf_extract::extract_text(path)
        .with_context(|| format!("extracting text from {}", path.display()))?;
    Ok(ExtractedPdf {
        text,
        page_count: None,
    })
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
                .map(|(i, _)| i + 1)
                .unwrap_or(buf.len());
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
