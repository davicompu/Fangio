//! Scopus CSV export parser. Uses the `csv` crate's flexible reader because
//! Scopus exports occasionally include trailing-comma rows and embedded commas
//! inside quoted titles.

use super::ParsedRecord;
use anyhow::Result;
use csv::ReaderBuilder;

pub fn parse(input: &str) -> Result<(Vec<ParsedRecord>, Vec<String>)> {
    let mut warnings = Vec::new();
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(input.as_bytes());

    let headers = rdr.headers()?.clone();
    let h = |name: &str| headers.iter().position(|c| c.eq_ignore_ascii_case(name));

    let i_authors = h("Authors").or_else(|| h("Author full names"));
    let i_title = h("Title").or_else(|| h("Document Title"));
    let i_year = h("Year");
    let i_source = h("Source title").or_else(|| h("Publication Name"));
    let i_abstract = h("Abstract");
    let i_doi = h("DOI");
    let i_link = h("Link").or_else(|| h("URL"));

    let mut out = Vec::new();
    for (lineno, result) in rdr.records().enumerate() {
        match result {
            Ok(row) => {
                let title = i_title
                    .and_then(|i| row.get(i))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                if title.is_empty() {
                    continue;
                }
                let authors = i_authors
                    .and_then(|i| row.get(i))
                    .map(|s| s.replace(',', "; "))
                    .filter(|s| !s.is_empty());
                let year = i_year
                    .and_then(|i| row.get(i))
                    .and_then(|s| s.trim().chars().take(4).collect::<String>().parse().ok());
                let journal = i_source.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
                let abstract_text = i_abstract.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
                let doi = i_doi.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
                let url = i_link.and_then(|i| row.get(i)).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
                out.push(ParsedRecord {
                    title,
                    authors,
                    year,
                    journal,
                    abstract_text,
                    doi,
                    url,
                    extras: vec![],
                });
            }
            Err(e) => {
                warnings.push(format!("row {}: {}", lineno + 2, e));
            }
        }
    }
    Ok((out, warnings))
}
