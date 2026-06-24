//! Deduplication.
//!
//! Two-pass design:
//!   1. **DOI exact match.** If two records share a normalized DOI, they're
//!      the same article — high confidence, auto-mergeable.
//!   2. **Title + year fuzzy.** Otherwise we cluster by normalized title (Jaro-
//!      Winkler ≥ 0.92) within the same publication year ±1. Researcher
//!      review required before merging.

use strsim::jaro_winkler;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone)]
pub struct DedupCandidate {
    pub article_id: i64,
    pub title: String,
    pub year: Option<i32>,
    pub doi: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DedupCluster {
    pub members: Vec<i64>,
    pub kind: ClusterKind,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ClusterKind {
    DoiExact,
    TitleFuzzy,
}

/// Normalize a title for fuzzy comparison: lowercase, strip diacritics,
/// keep only alphanumerics + single spaces.
pub fn normalize_title(s: &str) -> String {
    let nfd: String = s.nfd().filter(|c| !is_combining(*c)).collect();
    let lower = nfd.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut prev_space = true;
    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_space = false;
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    }
    out.trim().to_string()
}

fn is_combining(c: char) -> bool {
    matches!(c as u32, 0x0300..=0x036f | 0x1ab0..=0x1aff | 0x1dc0..=0x1dff | 0x20d0..=0x20ff | 0xfe20..=0xfe2f)
}

const FUZZY_THRESHOLD: f64 = 0.92;

/// Cluster a set of candidates. Returns clusters of size ≥ 2 (singletons are
/// not interesting). DOI clusters come first, then fuzzy clusters.
pub fn cluster(mut candidates: Vec<DedupCandidate>) -> Vec<DedupCluster> {
    let mut clusters = Vec::new();
    // Pass 1: DOI exact
    let mut by_doi: std::collections::HashMap<String, Vec<i64>> = std::collections::HashMap::new();
    let mut consumed: std::collections::HashSet<i64> = std::collections::HashSet::new();
    for c in &candidates {
        if let Some(doi) = &c.doi {
            by_doi.entry(doi.clone()).or_default().push(c.article_id);
        }
    }
    for (_doi, members) in by_doi {
        if members.len() >= 2 {
            for m in &members {
                consumed.insert(*m);
            }
            clusters.push(DedupCluster {
                members,
                kind: ClusterKind::DoiExact,
                confidence: 1.0,
            });
        }
    }
    // Pass 2: title fuzzy on the leftover
    candidates.retain(|c| !consumed.contains(&c.article_id));
    let normalized: Vec<String> = candidates.iter().map(|c| normalize_title(&c.title)).collect();

    let mut visited = vec![false; candidates.len()];
    for i in 0..candidates.len() {
        if visited[i] || normalized[i].is_empty() {
            continue;
        }
        let mut group = vec![candidates[i].article_id];
        let mut max_score = 0.0_f64;
        for j in (i + 1)..candidates.len() {
            if visited[j] || normalized[j].is_empty() {
                continue;
            }
            // Year guard: ±1 if both years present
            if let (Some(y1), Some(y2)) = (candidates[i].year, candidates[j].year) {
                if (y1 - y2).abs() > 1 {
                    continue;
                }
            }
            let s = jaro_winkler(&normalized[i], &normalized[j]);
            if s >= FUZZY_THRESHOLD {
                visited[j] = true;
                group.push(candidates[j].article_id);
                if s > max_score {
                    max_score = s;
                }
            }
        }
        if group.len() >= 2 {
            visited[i] = true;
            clusters.push(DedupCluster {
                members: group,
                kind: ClusterKind::TitleFuzzy,
                confidence: max_score as f32,
            });
        }
    }
    clusters
}
