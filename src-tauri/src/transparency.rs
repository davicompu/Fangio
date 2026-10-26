//! Pure helpers for AI-decision provenance.

use sha2::{Digest, Sha256};

/// First 12 lowercase hex chars of the SHA-256 of a system prompt.
pub fn prompt_fingerprint(system_prompt: &str) -> String {
    let digest = Sha256::digest(system_prompt.as_bytes());
    digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// Derived review status of one AI screening decision. `None` for error/skipped.
pub fn review_status(
    decision: &str,
    auto_applied: bool,
    overridden: bool,
    open_count: i64,
) -> Option<&'static str> {
    if matches!(decision, "error" | "skipped") {
        return None;
    }
    Some(if overridden {
        "overridden"
    } else if open_count > 0 {
        "opened"
    } else if auto_applied {
        "auto_applied"
    } else {
        "pending"
    })
}

/// Name recorded against open events: env `USER`, else "researcher".
pub fn researcher_name() -> String {
    std::env::var("USER")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "researcher".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_precedence() {
        assert_eq!(review_status("exclude", true, true, 3), Some("overridden"));
        assert_eq!(review_status("exclude", true, false, 1), Some("opened"));
        assert_eq!(review_status("exclude", true, false, 0), Some("auto_applied"));
        assert_eq!(review_status("uncertain", false, false, 0), Some("pending"));
    }

    #[test]
    fn errors_and_skips_have_no_status() {
        assert_eq!(review_status("error", false, false, 0), None);
        assert_eq!(review_status("skipped", false, false, 0), None);
    }

    #[test]
    fn fingerprint_is_12_hex_and_stable() {
        let a = prompt_fingerprint("You are X");
        assert_eq!(a.len(), 12);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(a, prompt_fingerprint("You are X"));
        assert_ne!(a, prompt_fingerprint("You are Y"));
    }
}
