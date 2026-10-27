//! Meaning-based (embedding) helpers for the Library search and the glossary.
//! Pure functions only; database access and model calls live in db.rs / lib.rs.

use sha2::{Digest, Sha256};

/// The text embedded for an article's title + abstract. Title alone when there
/// is no abstract, so every paper is searchable.
pub fn title_abstract_text(title: &str, abstract_text: Option<&str>) -> String {
    match abstract_text.map(str::trim).filter(|a| !a.is_empty()) {
        Some(a) => format!("{}\n\n{}", title.trim(), a),
        None => title.trim().to_string(),
    }
}

/// SHA-256 (hex) of the embedded text, so a changed title/abstract re-embeds.
pub fn text_hash(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    format!("{:x}", h.finalize())
}

/// Whether an article's stored title/abstract embedding must be (re)computed.
pub fn needs_embedding(stored: Option<(&str, &str)>, current_hash: &str, current_model: &str) -> bool {
    match stored {
        None => true,
        Some((hash, model)) => hash != current_hash || model != current_model,
    }
}

/// Number of clusters to build from `n` papers. `requested` is the
/// researcher's choice; without one, the long-standing automatic rule
/// (half the papers, at most 8). Always between 2 and `n`.
pub fn cluster_k(requested: Option<usize>, n: usize) -> usize {
    let k = requested.unwrap_or_else(|| (n / 2).min(8));
    k.max(2).min(n.max(2))
}

/// K-means over cosine similarity: centroids start at `k` evenly spaced items,
/// at most 20 iterations. Returns the cluster index of each item.
pub fn kmeans(items: &[Vec<f32>], k: usize) -> Vec<usize> {
    let n = items.len();
    if n == 0 || k == 0 {
        return vec![0; n];
    }
    let dim = items[0].len();
    let mut centroids: Vec<Vec<f32>> = (0..k).map(|i| items[i * n / k].clone()).collect();
    let mut assignments = vec![0usize; n];
    for _ in 0..20 {
        let mut changed = false;
        for (i, emb) in items.iter().enumerate() {
            let best = (0..k)
                .max_by(|&a, &b| {
                    crate::vector::cosine(emb, &centroids[a])
                        .partial_cmp(&crate::vector::cosine(emb, &centroids[b]))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0);
            if assignments[i] != best {
                changed = true;
            }
            assignments[i] = best;
        }
        if !changed {
            break;
        }
        let mut sums = vec![vec![0.0f32; dim]; k];
        let mut counts = vec![0usize; k];
        for (i, emb) in items.iter().enumerate() {
            let c = assignments[i];
            for j in 0..dim {
                sums[c][j] += emb[j];
            }
            counts[c] += 1;
        }
        for c in 0..k {
            if counts[c] > 0 {
                let n_c = counts[c] as f32;
                centroids[c] = sums[c].iter().map(|v| v / n_c).collect();
            }
        }
    }
    assignments
}

/// One paper's match for a meaning-based search.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PaperMatch {
    pub article_id: i64,
    /// Score used for ranking: the abstract score, or the best PDF passage
    /// score after removing the typical passage-over-abstract gap.
    pub score: f32,
    pub abstract_score: Option<f32>,
    pub passage_score: Option<f32>,
    /// The chunk behind `passage_score`.
    pub passage_chunk_id: Option<i64>,
}

/// Rank papers by similarity to `query`. `abstracts` are `(article_id, vector)`;
/// `chunks` are `(chunk_id, article_id, vector)`. A paper's best passage is
/// picked from many chunks, so it scores higher than its abstract on average;
/// that gap (mean over papers with both, never negative) is subtracted from
/// passage scores so papers with and without PDFs are ranked fairly.
pub fn rank_papers(
    query: &[f32],
    abstracts: &[(i64, Vec<f32>)],
    chunks: &[(i64, i64, Vec<f32>)],
) -> Vec<PaperMatch> {
    use std::collections::HashMap;
    let mut by_paper: HashMap<i64, PaperMatch> = HashMap::new();
    for (aid, v) in abstracts {
        let s = crate::vector::cosine(query, v);
        by_paper.insert(*aid, PaperMatch {
            article_id: *aid,
            score: s,
            abstract_score: Some(s),
            passage_score: None,
            passage_chunk_id: None,
        });
    }
    for (cid, aid, v) in chunks {
        let s = crate::vector::cosine(query, v);
        let m = by_paper.entry(*aid).or_insert(PaperMatch {
            article_id: *aid,
            score: f32::MIN,
            abstract_score: None,
            passage_score: None,
            passage_chunk_id: None,
        });
        if m.passage_score.map_or(true, |p| s > p) {
            m.passage_score = Some(s);
            m.passage_chunk_id = Some(*cid);
        }
    }
    let gaps: Vec<f32> = by_paper
        .values()
        .filter_map(|m| Some(m.passage_score? - m.abstract_score?))
        .collect();
    let gap = if gaps.is_empty() { 0.0 } else { (gaps.iter().sum::<f32>() / gaps.len() as f32).max(0.0) };
    let mut out: Vec<PaperMatch> = by_paper
        .into_values()
        .map(|mut m| {
            let adjusted = m.passage_score.map(|p| p - gap);
            m.score = match (m.abstract_score, adjusted) {
                (Some(a), Some(p)) => a.max(p),
                (Some(a), None) => a,
                (None, Some(p)) => p,
                (None, None) => f32::MIN,
            };
            m
        })
        .collect();
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal).then(a.article_id.cmp(&b.article_id)));
    out
}

/// Normalised form of a glossary term for matching decisions across
/// re-extractions: lowercase, straight apostrophes, single spaces.
pub fn term_key(term: &str) -> String {
    term.replace(['\u{2019}', '\u{2018}'], "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The two keys of a term pair in a fixed order.
pub fn pair_keys(a: &str, b: &str) -> (String, String) {
    let (ka, kb) = (term_key(a), term_key(b));
    if ka <= kb { (ka, kb) } else { (kb, ka) }
}

/// Pairs of items whose vectors are at least `threshold` similar, most
/// similar first: `(i, j, score)` with `i < j`.
pub fn merge_candidates(vectors: &[Vec<f32>], threshold: f32) -> Vec<(usize, usize, f32)> {
    let mut out = Vec::new();
    for i in 0..vectors.len() {
        for j in (i + 1)..vectors.len() {
            let s = crate::vector::cosine(&vectors[i], &vectors[j]);
            if s >= threshold {
                out.push((i, j, s));
            }
        }
    }
    out.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// Paper ids and aliases of `keep` after absorbing `other`: the union of both
/// paper lists (sorted), and keep's aliases plus other's name and aliases
/// (no duplicates, never keep's own name).
pub fn merged_term(
    keep_term: &str,
    keep_ids: &[i64],
    keep_aliases: &[String],
    other_term: &str,
    other_ids: &[i64],
    other_aliases: &[String],
) -> (Vec<i64>, Vec<String>) {
    let mut ids: Vec<i64> = keep_ids.iter().chain(other_ids).copied().collect();
    ids.sort_unstable();
    ids.dedup();
    let mut aliases: Vec<String> = Vec::new();
    for a in keep_aliases.iter().map(String::as_str).chain(std::iter::once(other_term)).chain(other_aliases.iter().map(String::as_str)) {
        let k = term_key(a);
        if k != term_key(keep_term) && !aliases.iter().any(|x| term_key(x) == k) {
            aliases.push(a.to_string());
        }
    }
    (ids, aliases)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_title_then_abstract_or_title_alone() {
        assert_eq!(title_abstract_text(" T ", Some(" A ")), "T\n\nA");
        assert_eq!(title_abstract_text("T", Some("   ")), "T");
        assert_eq!(title_abstract_text("T", None), "T");
    }

    #[test]
    fn hash_changes_with_text() {
        assert_eq!(text_hash("a"), text_hash("a"));
        assert_ne!(text_hash("a"), text_hash("b"));
        assert_eq!(text_hash("a").len(), 64);
    }

    #[test]
    fn re_embed_when_missing_text_changed_or_model_changed() {
        assert!(needs_embedding(None, "h", "m"));
        assert!(!needs_embedding(Some(("h", "m")), "h", "m"));
        assert!(needs_embedding(Some(("old", "m")), "h", "m"));
        assert!(needs_embedding(Some(("h", "other-model")), "h", "m"));
    }

    #[test]
    fn automatic_k_keeps_the_old_rule() {
        assert_eq!(cluster_k(None, 40), 8);
        assert_eq!(cluster_k(None, 10), 5);
        assert_eq!(cluster_k(None, 3), 2);
        assert_eq!(cluster_k(None, 2), 2);
    }

    #[test]
    fn chosen_k_is_kept_within_2_and_the_paper_count() {
        assert_eq!(cluster_k(Some(12), 40), 12);
        assert_eq!(cluster_k(Some(1), 40), 2);
        assert_eq!(cluster_k(Some(99), 40), 40);
    }

    #[test]
    fn kmeans_separates_two_obvious_groups() {
        let items = vec![
            vec![1.0, 0.0],
            vec![0.9, 0.1],
            vec![0.0, 1.0],
            vec![0.1, 0.9],
        ];
        let a = kmeans(&items, 2);
        assert_eq!(a[0], a[1]);
        assert_eq!(a[2], a[3]);
        assert_ne!(a[0], a[2]);
    }

    #[test]
    fn ranks_by_similarity_and_keeps_both_scores() {
        let q = vec![1.0, 0.0];
        let abstracts = vec![(1, vec![1.0, 0.0]), (2, vec![0.0, 1.0])];
        let r = rank_papers(&q, &abstracts, &[]);
        assert_eq!(r.iter().map(|m| m.article_id).collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(r[0].abstract_score, Some(1.0));
        assert_eq!(r[0].passage_score, None);
    }

    #[test]
    fn passage_gap_is_removed_so_pdf_papers_are_not_favoured() {
        let q = vec![1.0, 0.0];
        // Paper 1: abstract 0.6, best passage 0.8 → gap 0.2 (only paper with both).
        // Paper 2: abstract-only, 0.7 — should outrank paper 1 (0.8 - 0.2 = 0.6).
        let v = |c: f32| vec![c, (1.0 - c * c).sqrt()];
        let abstracts = vec![(1, v(0.6)), (2, v(0.7))];
        let chunks = vec![(10, 1, v(0.5)), (11, 1, v(0.8))];
        let r = rank_papers(&q, &abstracts, &chunks);
        assert_eq!(r[0].article_id, 2);
        assert_eq!(r[1].passage_chunk_id, Some(11));
        assert!((r[1].passage_score.unwrap() - 0.8).abs() < 1e-4);
        assert!((r[1].score - 0.6).abs() < 1e-4);
    }

    #[test]
    fn paper_with_only_pdf_text_is_still_ranked() {
        let q = vec![1.0, 0.0];
        let r = rank_papers(&q, &[], &[(5, 9, vec![1.0, 0.0])]);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].article_id, 9);
        assert_eq!(r[0].abstract_score, None);
    }

    #[test]
    fn term_keys_ignore_case_spacing_and_curly_quotes() {
        assert_eq!(term_key("Bloom\u{2019}s  Taxonomy"), "bloom's taxonomy");
        assert_eq!(pair_keys("Zeta", "alpha"), ("alpha".to_string(), "zeta".to_string()));
    }

    #[test]
    fn candidates_above_threshold_most_similar_first() {
        let v = vec![vec![1.0, 0.0], vec![0.99, 0.141], vec![0.0, 1.0], vec![0.95, 0.312]];
        let c = merge_candidates(&v, 0.9);
        assert_eq!(c.iter().map(|x| (x.0, x.1)).collect::<Vec<_>>(), vec![(0, 1), (1, 3), (0, 3)]);
    }

    #[test]
    fn merged_term_unites_papers_and_keeps_other_name_as_alias() {
        let (ids, aliases) = merged_term(
            "Generative AI", &[3, 1], &["GenAI".into()],
            "Generative Artificial Intelligence", &[2, 3], &["GAI".into(), "genai".into(), "Generative AI".into()],
        );
        assert_eq!(ids, vec![1, 2, 3]);
        assert_eq!(aliases, vec!["GenAI", "Generative Artificial Intelligence", "GAI"]);
    }
}
