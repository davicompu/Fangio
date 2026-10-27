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
}
