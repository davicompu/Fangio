//! Decision-log entries for the setup assistant ("Ask assistant" in onboarding).
//!
//! Each use of the assistant leaves linked, append-only entries that share a
//! `call_id`: the clarifying questions (AI), the suggestion (AI), and what the
//! researcher did next (human). The researcher's final answer for the step is
//! still logged separately under its own `project.*` key.

use crate::suggest::{QAndA, StepContextField, StepQuestion};
use serde_json::{json, Value};

pub const KEY_QUESTIONS: &str = "onboarding.assistant.questions";
pub const KEY_SUGGESTION: &str = "onboarding.assistant.suggestion";
pub const KEY_OUTCOME: &str = "onboarding.assistant.outcome";
/// Prefix shared by every setup-assistant entry.
pub const KEY_PREFIX: &str = "onboarding.assistant.";

/// What the researcher did after asking. `used`/`undone`/`ignored` apply to the
/// steps where a suggestion can replace the answer; `kept_choice` and
/// `changed_choice` to the review-type and framework steps, where the assistant
/// only gives advice; `closed_before_suggestion` when no suggestion was made.
pub const OUTCOMES: [&str; 6] = [
    "used",
    "undone",
    "ignored",
    "kept_choice",
    "changed_choice",
    "closed_before_suggestion",
];

pub fn new_call_id(step_id: &str) -> String {
    format!("{step_id}-{}", chrono::Utc::now().timestamp_micros())
}

/// AI entry: the clarifying questions, or the error if the call failed.
pub fn questions_entry(
    call_id: &str,
    step_id: &str,
    field_text: &str,
    context: &[StepContextField],
    questions: Result<&[StepQuestion], &str>,
    model: &str,
) -> Value {
    let mut v = json!({
        "call_id": call_id,
        "step": step_id,
        "field_text": field_text,
        "context": context,
        "model": model,
    });
    match questions {
        Ok(q) => v["questions"] = json!(q),
        Err(e) => v["error"] = json!(e),
    }
    v
}

/// AI entry: the suggestion (reason goes in the log row's rationale), with the
/// researcher's answers to the clarifying questions or the fact they skipped.
#[allow(clippy::too_many_arguments)]
pub fn suggestion_entry(
    call_id: &str,
    step_id: &str,
    field_text: &str,
    context: &[StepContextField],
    answers: &[QAndA],
    skipped: bool,
    suggestion: Result<&str, &str>,
    model: &str,
) -> Value {
    let mut v = json!({
        "call_id": call_id,
        "step": step_id,
        "field_text": field_text,
        "context": context,
        "answers": answers,
        "skipped": skipped,
        "model": model,
    });
    match suggestion {
        Ok(s) => v["suggestion"] = json!(s),
        Err(e) => v["error"] = json!(e),
    }
    v
}

/// Human entry: what the researcher did after the suggestion.
pub fn outcome_entry(
    call_id: &str,
    step_id: &str,
    outcome: &str,
    final_value: Option<&str>,
) -> Result<Value, String> {
    if !OUTCOMES.contains(&outcome) {
        return Err(format!("unknown setup assistant outcome: {outcome}"));
    }
    Ok(json!({
        "call_id": call_id,
        "step": step_id,
        "outcome": outcome,
        "final_value": final_value,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Vec<StepContextField> {
        vec![StepContextField { label: "Title".into(), value: "Biases in AI".into() }]
    }

    #[test]
    fn questions_entry_records_field_context_questions_and_model() {
        let q = vec![StepQuestion { q: "Which level?".into(), options: vec!["UG".into(), "PG".into()] }];
        let v = questions_entry("c1", "domain", "AI in education", &ctx(), Ok(&q), "qwen3-8b");
        assert_eq!(v["call_id"], "c1");
        assert_eq!(v["step"], "domain");
        assert_eq!(v["field_text"], "AI in education");
        assert_eq!(v["context"][0]["label"], "Title");
        assert_eq!(v["context"][0]["value"], "Biases in AI");
        assert_eq!(v["questions"][0]["q"], "Which level?");
        assert_eq!(v["questions"][0]["options"][1], "PG");
        assert_eq!(v["model"], "qwen3-8b");
        assert!(v.get("error").is_none());
    }

    #[test]
    fn failed_call_records_the_error() {
        let v = questions_entry("c1", "name", "x", &[], Err("connection refused"), "m");
        assert_eq!(v["error"], "connection refused");
        assert!(v.get("questions").is_none());
    }

    #[test]
    fn suggestion_entry_records_answers_or_skip() {
        let a = vec![QAndA { q: "Which level?".into(), a: "PG".into() }];
        let v = suggestion_entry("c1", "rq_what", "What?", &ctx(), &a, false, Ok("What is known…?"), "m");
        assert_eq!(v["answers"][0]["a"], "PG");
        assert_eq!(v["skipped"], false);
        assert_eq!(v["suggestion"], "What is known…?");

        let v = suggestion_entry("c2", "rq_what", "What?", &ctx(), &[], true, Ok("S"), "m");
        assert_eq!(v["skipped"], true);
        assert_eq!(v["answers"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn outcome_entry_validates_outcome() {
        let v = outcome_entry("c1", "name", "ignored", Some("My own title")).unwrap();
        assert_eq!(v["outcome"], "ignored");
        assert_eq!(v["final_value"], "My own title");
        assert!(outcome_entry("c1", "name", "approved", None).is_err());
    }
}
