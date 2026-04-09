//! Web of Science tab-delimited "Full Record" export.
//!
//! WoS exports begin with "FN <file name>\n" and "VR <version>\n", followed
//! by a header line of two-letter field tags separated by tabs, then one line
//! per record. Some records use multi-line fields prefixed by spaces — we
//! collapse them onto the parent line.

use super::ParsedRecord;
use anyhow::{anyhow, Result};

pub fn parse(input: &str) -> Result<(Vec<ParsedRecord>, Vec<String>)> {
    let warnings = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() {
        return Ok((vec![], warnings));
    }
    // Find header line: starts with "PT\t"
    let header_idx = lines
        .iter()
        .position(|l| l.starts_with("PT\t") || l.contains("\tPT\t"))
        .ok_or_else(|| anyhow!("Web of Science file: header row (PT\\t...) not found"))?;
    let header: Vec<&str> = lines[header_idx].split('\t').collect();

    // Map field code -> column index
    let col = |code: &str| header.iter().position(|h| *h == code);
    let i_ti = col("TI");
    let i_au = col("AU").or_else(|| col("AF"));
    let i_py = col("PY");
    let i_so = col("SO");
    let i_ab = col("AB");
    let i_di = col("DI");

    let mut out = Vec::new();
    let mut current_row: Option<Vec<String>> = None;
    for line in &lines[header_idx + 1..] {
        if line.is_empty() {
            continue;
        }
        if line == &"EF" {
            break;
        }
        // Records are tab-separated and exactly the header width
        let cells: Vec<String> = line.split('\t').map(|s| s.to_string()).collect();
        if cells.len() == header.len() {
            if let Some(prev) = current_row.take() {
                if let Some(rec) = build_record(&prev, i_ti, i_au, i_py, i_so, i_ab, i_di) {
                    out.push(rec);
                }
            }
            current_row = Some(cells);
        } else if let Some(ref mut row) = current_row {
            // Continuation: append to last cell
            if let Some(last) = row.last_mut() {
                last.push(' ');
                last.push_str(line.trim());
            }
        }
    }
    if let Some(prev) = current_row.take() {
        if let Some(rec) = build_record(&prev, i_ti, i_au, i_py, i_so, i_ab, i_di) {
            out.push(rec);
        }
    }
    Ok((out, warnings))
}

#[allow(clippy::too_many_arguments)]
fn build_record(
    row: &[String],
    i_ti: Option<usize>,
    i_au: Option<usize>,
    i_py: Option<usize>,
    i_so: Option<usize>,
    i_ab: Option<usize>,
    i_di: Option<usize>,
) -> Option<ParsedRecord> {
    let title = i_ti.and_then(|i| row.get(i)).map(|s| s.trim().to_string())?;
    if title.is_empty() {
        return None;
    }
    let authors = i_au
        .and_then(|i| row.get(i))
        .map(|s| s.replace(';', "; "))
        .filter(|s| !s.is_empty());
    let year = i_py
        .and_then(|i| row.get(i))
        .and_then(|s| s.trim().chars().take(4).collect::<String>().parse().ok());
    let journal = i_so.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let abstract_text = i_ab.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let doi = i_di.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    Some(ParsedRecord {
        title,
        authors,
        year,
        journal,
        abstract_text,
        doi,
        url: None,
        extras: vec![],
    })
}
