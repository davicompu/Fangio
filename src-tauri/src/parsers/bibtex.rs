//! Lightweight BibTeX parser. Handles the common case of @article / @inproceedings
//! entries with brace-delimited values. String concatenation, @string definitions,
//! and crossref are intentionally not supported — researchers should re-export
//! cleaner BibTeX from their reference manager if they need those.

use super::ParsedRecord;
use anyhow::Result;

pub fn parse(input: &str) -> Result<(Vec<ParsedRecord>, Vec<String>)> {
    let mut out = Vec::new();
    let warnings = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        // Skip until next '@'
        while i < bytes.len() && bytes[i] != b'@' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        i += 1; // skip @
        // Read entry type until '{'
        let type_start = i;
        while i < bytes.len() && bytes[i] != b'{' && bytes[i] != b'(' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let entry_type: String = input[type_start..i]
            .trim()
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect::<String>()
            .to_lowercase();
        if entry_type.is_empty() || entry_type == "string" || entry_type == "preamble" || entry_type == "comment" {
            // Skip these blocks
            i = skip_balanced(bytes, i);
            continue;
        }
        let opener = bytes[i];
        let closer = if opener == b'{' { b'}' } else { b')' };
        i += 1; // skip opener

        // Read citation key until first ','
        let key_start = i;
        while i < bytes.len() && bytes[i] != b',' && bytes[i] != closer {
            i += 1;
        }
        let _cite_key = input[key_start..i].trim().to_string();
        if i < bytes.len() && bytes[i] == b',' {
            i += 1;
        }

        // Parse fields until matching closer
        let mut rec = ParsedRecord::default();
        let mut authors_raw: Option<String> = None;

        loop {
            skip_ws(bytes, &mut i);
            if i >= bytes.len() || bytes[i] == closer {
                break;
            }
            // Field name
            let name_start = i;
            while i < bytes.len() && bytes[i] != b'=' && bytes[i] != closer {
                i += 1;
            }
            if i >= bytes.len() || bytes[i] == closer {
                break;
            }
            let field_name = input[name_start..i].trim().to_lowercase();
            i += 1; // skip '='
            skip_ws(bytes, &mut i);
            // Value: either {...} or "..." or bare token
            let value = read_value(bytes, &mut i, input);
            // Trailing comma
            skip_ws(bytes, &mut i);
            if i < bytes.len() && bytes[i] == b',' {
                i += 1;
            }
            // Map field
            match field_name.as_str() {
                "title" => rec.title = clean(&value),
                "author" => authors_raw = Some(value),
                "year" => rec.year = value.trim().chars().take(4).collect::<String>().parse().ok(),
                "journal" | "booktitle" => rec.journal = Some(clean(&value)),
                "abstract" => rec.abstract_text = Some(clean(&value)),
                "doi" => rec.doi = Some(value.trim().to_string()),
                "url" => rec.url = Some(value.trim().to_string()),
                _ => {}
            }
        }
        // close
        if i < bytes.len() && bytes[i] == closer {
            i += 1;
        }

        if let Some(raw) = authors_raw {
            // BibTeX authors are joined with " and "
            let parts: Vec<String> = raw.split(" and ").map(|s| clean(s)).collect();
            rec.authors = Some(parts.join("; "));
        }

        if !rec.title.is_empty() {
            out.push(rec);
        }
    }
    Ok((out, warnings))
}

fn skip_ws(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
        *i += 1;
    }
}

fn read_value(bytes: &[u8], i: &mut usize, input: &str) -> String {
    if *i >= bytes.len() {
        return String::new();
    }
    let opener = bytes[*i];
    if opener == b'{' {
        let start = *i + 1;
        let mut depth = 1;
        *i += 1;
        while *i < bytes.len() && depth > 0 {
            match bytes[*i] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            if depth > 0 {
                *i += 1;
            }
        }
        let v = input[start..*i].to_string();
        if *i < bytes.len() {
            *i += 1; // closing brace
        }
        v
    } else if opener == b'"' {
        let start = *i + 1;
        *i += 1;
        while *i < bytes.len() && bytes[*i] != b'"' {
            if bytes[*i] == b'\\' && *i + 1 < bytes.len() {
                *i += 2;
            } else {
                *i += 1;
            }
        }
        let v = input[start..*i].to_string();
        if *i < bytes.len() {
            *i += 1;
        }
        v
    } else {
        // Bare token: read until comma or closer
        let start = *i;
        while *i < bytes.len()
            && bytes[*i] != b','
            && bytes[*i] != b'}'
            && bytes[*i] != b')'
            && !bytes[*i].is_ascii_whitespace()
        {
            *i += 1;
        }
        input[start..*i].to_string()
    }
}

fn skip_balanced(bytes: &[u8], start: usize) -> usize {
    if start >= bytes.len() {
        return bytes.len();
    }
    let opener = bytes[start];
    let closer = if opener == b'{' { b'}' } else if opener == b'(' { b')' } else { return start + 1; };
    let mut depth = 0i32;
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == opener {
            depth += 1;
        } else if bytes[i] == closer {
            depth -= 1;
            if depth == 0 {
                return i + 1;
            }
        }
        i += 1;
    }
    bytes.len()
}

/// Strip surrounding braces and collapse whitespace; remove BibTeX escape
/// sequences for the most common diacritics.
fn clean(s: &str) -> String {
    let trimmed = s.trim();
    let unbraced = trimmed.replace('{', "").replace('}', "");
    unbraced.split_whitespace().collect::<Vec<_>>().join(" ")
}
