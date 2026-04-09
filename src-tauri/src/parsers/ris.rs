//! RIS bibliographic format parser.
//!
//! Each record begins with `TY  - <type>` and ends with `ER  - `. Field tags
//! are 2-letter codes followed by `  - ` and a value, optionally continued on
//! subsequent lines (we keep the simple case: each tag-line is one value, and
//! repeated AU tags accumulate authors).

use super::ParsedRecord;
use anyhow::Result;

pub fn parse(input: &str) -> Result<(Vec<ParsedRecord>, Vec<String>)> {
    let mut out = Vec::new();
    let mut warnings = Vec::new();
    let mut current: Option<RisRecord> = None;

    for (lineno, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        // Tag pattern: "XX  - value" or "XX - value"
        let (tag, value) = match split_tag(line) {
            Some(t) => t,
            None => {
                // Continuation lines (not strictly RIS, but tolerate)
                if let Some(ref mut r) = current {
                    if !r.last_field.is_empty() {
                        match r.last_field.as_str() {
                            "AB" | "N2" => {
                                if let Some(ab) = r.abstract_text.as_mut() {
                                    ab.push(' ');
                                    ab.push_str(line.trim());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                continue;
            }
        };
        let tag = tag.to_string();
        let value = value.trim().to_string();

        if tag == "TY" {
            // start a new record
            if let Some(prev) = current.take() {
                if let Some(rec) = prev.finalize() {
                    out.push(rec);
                }
            }
            current = Some(RisRecord::default());
        }
        if let Some(rec) = current.as_mut() {
            rec.last_field = tag.clone();
            match tag.as_str() {
                "TI" | "T1" => rec.title = Some(value),
                "AU" | "A1" => rec.authors.push(value),
                "PY" | "Y1" | "DA" => {
                    if rec.year.is_none() {
                        rec.year = parse_year(&value);
                    }
                }
                "JF" | "JO" | "T2" => {
                    if rec.journal.is_none() {
                        rec.journal = Some(value);
                    }
                }
                "AB" | "N2" => rec.abstract_text = Some(value),
                "DO" => rec.doi = Some(value),
                "UR" | "L1" | "L2" => {
                    if rec.url.is_none() {
                        rec.url = Some(value);
                    }
                }
                "ER" => {
                    if let Some(prev) = current.take() {
                        if let Some(r) = prev.finalize() {
                            out.push(r);
                        }
                    }
                }
                _ => {}
            }
        } else {
            warnings.push(format!("line {}: stray tag '{}' before any TY", lineno + 1, tag));
        }
    }
    // Trailing record without ER
    if let Some(prev) = current.take() {
        if let Some(r) = prev.finalize() {
            out.push(r);
        }
    }
    Ok((out, warnings))
}

fn split_tag(line: &str) -> Option<(&str, &str)> {
    if line.len() < 6 {
        return None;
    }
    let tag = &line[..2];
    if !tag.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let rest = &line[2..];
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('-')?.trim_start();
    Some((tag, rest))
}

fn parse_year(value: &str) -> Option<i32> {
    let digits: String = value.chars().take(4).collect();
    digits.parse().ok()
}

#[derive(Default)]
struct RisRecord {
    title: Option<String>,
    authors: Vec<String>,
    year: Option<i32>,
    journal: Option<String>,
    abstract_text: Option<String>,
    doi: Option<String>,
    url: Option<String>,
    last_field: String,
}

impl RisRecord {
    fn finalize(self) -> Option<ParsedRecord> {
        let title = self.title?.trim().to_string();
        if title.is_empty() {
            return None;
        }
        let authors = if self.authors.is_empty() {
            None
        } else {
            Some(self.authors.join("; "))
        };
        Some(ParsedRecord {
            title,
            authors,
            year: self.year,
            journal: self.journal,
            abstract_text: self.abstract_text,
            doi: self.doi,
            url: self.url,
            extras: vec![],
        })
    }
}
