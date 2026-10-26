//! Pure helpers for AI-decision provenance.

use crate::suggest::{DimensionCoding, Suggestion};
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

/// AI screening stage whose log row describes the decision being cleared.
pub fn reverted_ai_stage(kind: &str, fulltext_status: Option<&str>) -> &'static str {
    if kind == "reset" || fulltext_status == Some("excluded") {
        "fulltext"
    } else {
        "title_abstract"
    }
}

/// Values attributed to the previous decision in a revert/reset event.
#[derive(Debug, PartialEq, Default)]
pub struct RevertAttribution {
    pub criterion: Option<i64>,
    pub reason: Option<String>,
    pub confidence: Option<f64>,
    pub model: Option<String>,
}

/// Latest real (non error/skipped) AI row for the stage, reduced to what is needed.
pub struct AiRowFacts<'a> {
    pub criterion: Option<i64>,
    pub rationale: &'a str,
    pub confidence: Option<f64>,
    pub model: Option<&'a str>,
}

/// Decide which criterion/reason/confidence/model to log. AI confidence and
/// model come only from the stage's AI row; criterion and human reason come
/// from the article only when the previous state at that stage was an exclusion.
pub fn attribute_reverted_decision(
    kind: &str,
    article_status: &str,
    fulltext_status: Option<&str>,
    article_criterion: Option<i64>,
    human_reason: Option<&str>,
    ai: Option<AiRowFacts>,
) -> RevertAttribution {
    let was_exclusion = if kind == "reset" {
        fulltext_status == Some("excluded")
    } else {
        article_status == "excluded" || fulltext_status == Some("excluded")
    };
    let non_blank = |s: &str| if s.trim().is_empty() { None } else { Some(s.to_string()) };
    let ai_reason = ai.as_ref().and_then(|a| non_blank(a.rationale));
    let ai_crit = ai.as_ref().and_then(|a| a.criterion);
    let (criterion, reason) = if was_exclusion {
        (
            article_criterion.or(ai_crit),
            human_reason.and_then(non_blank).or(ai_reason),
        )
    } else {
        (ai_crit, ai_reason)
    };
    RevertAttribution {
        criterion,
        reason,
        confidence: ai.as_ref().and_then(|a| a.confidence),
        model: ai.as_ref().and_then(|a| a.model.map(|m| m.to_string())),
    }
}

/// Truncate to at most `max` chars on a char boundary.
pub fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Detail line logged when the researcher answers an AI-review reminder.
pub fn reminder_detail(context: &str, auto: i64, pending: i64) -> String {
    format!("Before {context}: {auto} applied automatically, {pending} pending")
}


// ── Coding: theme matching, rater resolution, disposition ──────────────────

/// Crude suffix stripping so "tests"/"testing"/"tested" collapse to one stem,
/// and the English y→ies plural collapses too ("theories"→"theory",
/// "studies"→"study") so it matches the singular.
fn theme_stem(token: &str) -> String {
    // y→ies plural must be handled before the generic "es"/"s" rules, which
    // would otherwise turn "theories" into "theori" (≠ "theory").
    if token.len() > 4 && token.ends_with("ies") {
        return format!("{}y", &token[..token.len() - 3]);
    }
    for suf in ["ing", "ed", "es", "s"] {
        if token.len() > suf.len() + 2 && token.ends_with(suf) {
            return token[..token.len() - suf.len()].to_string();
        }
    }
    token.to_string()
}

/// Lowercase, drop punctuation, split on whitespace, stem each token.
fn theme_tokens(s: &str) -> Vec<String> {
    let lowered: String = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    lowered.split_whitespace().map(theme_stem).collect()
}

/// True when two theme labels refer to the same theme despite casing or minor
/// wording differences: identical token streams, one token set contained in the
/// other (qualifier words), or high token overlap (Jaccard ≥ 0.6).
pub fn themes_match(a: &str, b: &str) -> bool {
    let ta_v = theme_tokens(a);
    let tb_v = theme_tokens(b);
    if ta_v.is_empty() || tb_v.is_empty() {
        return ta_v == tb_v;
    }
    if ta_v == tb_v {
        return true;
    }
    let ta: std::collections::HashSet<&String> = ta_v.iter().collect();
    let tb: std::collections::HashSet<&String> = tb_v.iter().collect();
    // Containment (e.g. "information management" ⊂ "information management context"),
    // but require ≥2 shared-side tokens so single words don't swallow phrases.
    if ta.len().min(tb.len()) >= 2 && (ta.is_subset(&tb) || tb.is_subset(&ta)) {
        return true;
    }
    let inter = ta.intersection(&tb).count();
    let union = ta.union(&tb).count();
    union > 0 && (inter as f32 / union as f32) >= 0.6
}

/// Canonical display casing for a theme: trim and uppercase the first letter,
/// leaving the rest untouched so acronyms (LLM, GenAI) survive. Keeps every
/// stored theme consistently capitalised.
pub fn canonical_theme(s: &str) -> String {
    let t = s.trim();
    let mut chars = t.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}


/// One agent's vote for a code: its reason, model, confidence and call id.
#[derive(Clone, Debug, Default)]
pub struct AiCodeVote {
    pub rationale: Option<String>,
    pub model_name: Option<String>,
    pub confidence: Option<f64>,
    pub ai_call_id: Option<i64>,
}

/// A code after matching the two raters' outputs for one article.
#[derive(Clone, Debug)]
pub struct ResolvedCode {
    pub dimension: String,
    /// `canonical_theme` applied.
    pub code: String,
    pub r1: Option<AiCodeVote>,
    pub r2: Option<AiCodeVote>,
}

/// Match rater 1's codes against rater 2's (fuzzy, case-insensitive). Agreed
/// codes carry both votes; the label comes from rater 1. With no rater 2 every
/// code has only `r1`.
pub fn resolve_rater_codes(
    r1: &[DimensionCoding],
    r1_vote: impl Fn(&Suggestion) -> AiCodeVote,
    r2: Option<&[DimensionCoding]>,
    r2_vote: impl Fn(&Suggestion) -> AiCodeVote,
) -> Vec<ResolvedCode> {
    let mut out = Vec::new();
    let Some(r2) = r2 else {
        for dc in r1 {
            for s in &dc.suggestions {
                out.push(ResolvedCode {
                    dimension: dc.dimension.clone(),
                    code: canonical_theme(&s.code),
                    r1: Some(r1_vote(s)),
                    r2: None,
                });
            }
        }
        return out;
    };
    for dc in r1 {
        let r2_sugg: &[Suggestion] = r2
            .iter()
            .find(|d| d.dimension == dc.dimension)
            .map(|d| d.suggestions.as_slice())
            .unwrap_or(&[]);
        for s in &dc.suggestions {
            let m = r2_sugg.iter().find(|c| themes_match(&c.code, &s.code));
            out.push(ResolvedCode {
                dimension: dc.dimension.clone(),
                code: canonical_theme(&s.code),
                r1: Some(r1_vote(s)),
                r2: m.map(&r2_vote),
            });
        }
        let r1_codes: Vec<&str> = dc.suggestions.iter().map(|s| s.code.as_str()).collect();
        for s in r2_sugg {
            if !r1_codes.iter().any(|c| themes_match(c, &s.code)) {
                out.push(ResolvedCode {
                    dimension: dc.dimension.clone(),
                    code: canonical_theme(&s.code),
                    r1: None,
                    r2: Some(r2_vote(s)),
                });
            }
        }
    }
    out
}

#[derive(Debug, PartialEq, Eq)]
pub enum Disposition {
    ApplyCoding { source: &'static str },
    Propose { agreement: &'static str, rater: i64 },
}

/// What to do with a resolved code under the agreed-codes mode.
pub fn coding_disposition(mode: &str, dual: bool, rc: &ResolvedCode) -> Disposition {
    let suggest = mode == "suggest";
    let rater = if rc.r1.is_some() { 1 } else { 2 };
    if !dual {
        return if suggest {
            Disposition::Propose { agreement: "one", rater: 1 }
        } else {
            Disposition::ApplyCoding { source: "model-ai-consensus" }
        };
    }
    if rc.r1.is_some() && rc.r2.is_some() {
        if suggest {
            Disposition::Propose { agreement: "both", rater: 1 }
        } else {
            Disposition::ApplyCoding { source: "model-ai-consensus" }
        }
    } else {
        Disposition::Propose { agreement: "one", rater }
    }
}

/// True when the merging/consolidating model is also one of the coding agents
/// (case-insensitive, trimmed; a blank merging/consolidating model never matches).
#[allow(dead_code)] // mirrored in the modal (TS); kept here as the tested reference
pub fn self_preference_warning(merging_or_consolidating: &str, rater1: &str, rater2: Option<&str>) -> bool {
    let m = merging_or_consolidating.trim().to_lowercase();
    if m.is_empty() {
        return false;
    }
    m == rater1.trim().to_lowercase() || rater2.map_or(false, |r| m == r.trim().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminder_detail_text() {
        assert_eq!(
            reminder_detail("exporting docx", 3, 2),
            "Before exporting docx: 3 applied automatically, 2 pending"
        );
    }

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
    fn reset_of_human_fulltext_keep_has_no_ai_attribution() {
        // Only a title/abstract AI row exists, so no fulltext row is passed.
        let a = attribute_reverted_decision("reset", "included", Some("included"), None, None, None);
        assert_eq!(a, RevertAttribution::default());
        let (d, r) = describe_reverted_decision("reset", 5, "included", Some("included"), a.criterion, a.reason.as_deref(), a.confidence, a.model.as_deref());
        assert!(!d.contains("AI confidence") && !d.contains("model"));
        assert!(r.is_none());
        assert_eq!(reverted_ai_stage("reset", Some("included")), "fulltext");
    }

    #[test]
    fn reset_of_ai_fulltext_exclusion_carries_fulltext_row() {
        let ai = AiRowFacts { criterion: Some(2), rationale: "No PDF match", confidence: Some(0.91), model: Some("qwen") };
        let a = attribute_reverted_decision("reset", "excluded", Some("excluded"), Some(2), Some(""), Some(ai));
        assert_eq!(a.confidence, Some(0.91));
        assert_eq!(a.model.as_deref(), Some("qwen"));
        assert_eq!(a.reason.as_deref(), Some("No PDF match"));
        assert_eq!(a.criterion, Some(2));
    }

    #[test]
    fn revert_uses_human_reason_for_exclusion_and_stage_follows_fulltext() {
        let a = attribute_reverted_decision("revert", "excluded", None, Some(4), Some("Wrong population"), None);
        assert_eq!(a.reason.as_deref(), Some("Wrong population"));
        assert_eq!(a.criterion, Some(4));
        assert_eq!(reverted_ai_stage("revert", Some("excluded")), "fulltext");
        assert_eq!(reverted_ai_stage("revert", None), "title_abstract");
        // Not an exclusion: article criterion/reason ignored.
        let b = attribute_reverted_decision("revert", "included", None, Some(4), Some("stale"), None);
        assert_eq!(b, RevertAttribution::default());
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

    fn v() -> AiCodeVote { AiCodeVote::default() }
    fn sg(code: &str, rat: &str, conf: f64) -> Suggestion {
        Suggestion { code: code.into(), rationale: Some(rat.into()), is_new: false, confidence: Some(conf) }
    }
    fn vote(s: &Suggestion) -> AiCodeVote {
        AiCodeVote { rationale: s.rationale.clone(), model_name: None, confidence: s.confidence, ai_call_id: None }
    }

    #[test]
    fn suggestion_mode_saves_nothing_to_coding() {
        let rc = ResolvedCode { dimension: "Method".into(), code: "Survey".into(), r1: Some(v()), r2: Some(v()) };
        assert_eq!(coding_disposition("suggest", true, &rc), Disposition::Propose { agreement: "both", rater: 1 });
        let single = ResolvedCode { r2: None, ..rc.clone() };
        assert_eq!(coding_disposition("suggest", false, &single), Disposition::Propose { agreement: "one", rater: 1 });
        let only2 = ResolvedCode { r1: None, ..rc.clone() };
        assert_eq!(coding_disposition("suggest", true, &only2), Disposition::Propose { agreement: "one", rater: 2 });
    }

    #[test]
    fn auto_mode_matches_previous_behaviour() {
        let both = ResolvedCode { dimension: "M".into(), code: "S".into(), r1: Some(v()), r2: Some(v()) };
        assert_eq!(coding_disposition("auto", true, &both), Disposition::ApplyCoding { source: "model-ai-consensus" });
        let only1 = ResolvedCode { r2: None, ..both.clone() };
        assert_eq!(coding_disposition("auto", true, &only1), Disposition::Propose { agreement: "one", rater: 1 });
        let only2 = ResolvedCode { r1: None, ..both.clone() };
        assert_eq!(coding_disposition("auto", true, &only2), Disposition::Propose { agreement: "one", rater: 2 });
        assert_eq!(coding_disposition("auto", false, &only1), Disposition::ApplyCoding { source: "model-ai-consensus" });
    }

    #[test]
    fn agreed_code_keeps_both_votes() {
        let r1 = vec![DimensionCoding { dimension: "Method".into(), suggestions: vec![sg("Structural equation modeling", "A", 0.8)] }];
        let r2 = vec![DimensionCoding { dimension: "Method".into(), suggestions: vec![sg("structural equation models", "B", 0.6)] }];
        let out = resolve_rater_codes(&r1, vote, Some(&r2), vote);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].code, "Structural equation modeling");
        assert_eq!(out[0].r1.as_ref().unwrap().rationale.as_deref(), Some("A"));
        assert_eq!(out[0].r2.as_ref().unwrap().rationale.as_deref(), Some("B"));
        assert_eq!(out[0].r2.as_ref().unwrap().confidence, Some(0.6));
    }

    #[test]
    fn unmatched_codes_from_each_rater_are_single_votes() {
        let r1 = vec![DimensionCoding { dimension: "M".into(), suggestions: vec![sg("alpha thing", "A", 0.5)] }];
        let r2 = vec![DimensionCoding { dimension: "M".into(), suggestions: vec![sg("beta other", "B", 0.5)] }];
        let out = resolve_rater_codes(&r1, vote, Some(&r2), vote);
        assert_eq!(out.len(), 2);
        assert!(out[0].r1.is_some() && out[0].r2.is_none());
        assert!(out[1].r1.is_none() && out[1].r2.is_some());
    }

    #[test]
    fn warning_only_on_match() {
        assert!(self_preference_warning("qwen3-8b", "Qwen3-8B ", None));
        assert!(self_preference_warning("gemma", "qwen", Some("gemma")));
        assert!(!self_preference_warning("gemma", "qwen", Some("llama")));
        assert!(!self_preference_warning("", "qwen", None));
    }
}
