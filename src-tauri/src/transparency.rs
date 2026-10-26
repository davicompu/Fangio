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

/// Article change to apply for a returned screening decision, and whether it
/// was applied automatically. `None` leaves the article untouched (purification).
pub fn screening_outcome(
    decision: &str,
    confidence: f64,
    threshold: f64,
    exclude_only: bool,
) -> (Option<&'static str>, bool) {
    let confident = confidence >= threshold;
    if decision == "exclude" && confident {
        (Some("exclude"), true)
    } else if exclude_only {
        (None, false)
    } else if decision == "include" && confident {
        (Some("include"), true)
    } else {
        (Some("uncertain"), false)
    }
}

/// Detail line and rationale for a revert (title/abstract) or reset (full text)
/// review event. Parts that are `None` are left out; the rationale is the
/// previous reason verbatim.
#[allow(clippy::too_many_arguments)]
pub fn describe_reverted_decision(
    kind: &str,
    article_id: i64,
    prev_status: &str,
    prev_fulltext: Option<&str>,
    prev_criterion: Option<i64>,
    prev_reason: Option<&str>,
    prev_confidence: Option<f64>,
    prev_ai_model: Option<&str>,
) -> (String, Option<String>) {
    let mut parts: Vec<String> = Vec::new();
    match (kind, prev_fulltext) {
        ("reset", Some(ft)) => parts.push(format!("{ft} at full text")),
        _ => parts.push(prev_status.to_string()),
    }
    if let Some(c) = prev_criterion {
        parts.push(format!("criterion {c}"));
    }
    if let Some(c) = prev_confidence {
        parts.push(format!("AI confidence {}%", (c * 100.0).round() as i64));
    }
    if let Some(m) = prev_ai_model {
        parts.push(format!("model {m}"));
    }
    let head = if kind == "reset" {
        "Reset full-text decision"
    } else {
        "Reverted title/abstract decision"
    };
    (
        format!("{head} on article {article_id} (was: {})", parts.join(", ")),
        prev_reason.map(|s| s.to_string()),
    )
}

/// Truncate to at most `max` chars on a char boundary.
pub fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_matches_previous_threshold_rules() {
        assert_eq!(screening_outcome("exclude", 0.90, 0.90, false), (Some("exclude"), true));
        assert_eq!(screening_outcome("exclude", 0.89, 0.90, false), (Some("uncertain"), false));
        assert_eq!(screening_outcome("include", 0.95, 0.90, false), (Some("include"), true));
        assert_eq!(screening_outcome("include", 0.95, 0.90, true), (None, false));
        assert_eq!(screening_outcome("exclude", 0.95, 0.90, true), (Some("exclude"), true));
        assert_eq!(screening_outcome("uncertain", 0.99, 0.90, false), (Some("uncertain"), false));
    }

    #[test]
    fn revert_detail_keeps_old_ai_reason_and_confidence() {
        let (d, r) = describe_reverted_decision("revert", 12, "excluded", None, Some(4), Some("Off-topic: K-12 sample"), Some(0.93), Some("qwen3-8b"));
        assert_eq!(d, "Reverted title/abstract decision on article 12 (was: excluded, criterion 4, AI confidence 93%, model qwen3-8b)");
        assert_eq!(r.as_deref(), Some("Off-topic: K-12 sample"));
    }

    #[test]
    fn reset_detail_for_fulltext() {
        let (d, _) = describe_reverted_decision("reset", 3, "excluded", Some("excluded"), None, None, None, None);
        assert_eq!(d, "Reset full-text decision on article 3 (was: excluded at full text)");
    }

    #[test]
    fn truncates_on_char_boundary() {
        assert_eq!(truncate_chars("ééé", 2), "éé");
    }

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
