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
}
