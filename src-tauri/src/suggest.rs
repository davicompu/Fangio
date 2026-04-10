//! Local-model code suggestion for the arranging stage.
//!
//! Builds a focused, ≤80-line prompt and calls LM Studio. The model is asked
//! to return JSON; we tolerate prose around the JSON and extract the first
//! array we find. Reasoning traces are stripped upstream by the LM Studio
//! client.

use crate::lmstudio::{self, ChatMessage};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Suggestion {
    pub code: String,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub is_new: bool,
}

#[derive(Debug, Serialize)]
pub struct SuggestResult {
    pub suggestions: Vec<Suggestion>,
    pub model: Option<String>,
    pub latency_ms: u64,
    pub raw: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SuggestQueryResult {
    pub query: String,
    pub rationale: String,
    pub model: Option<String>,
    pub latency_ms: u64,
}

#[allow(clippy::too_many_arguments)]
pub async fn suggest_codes(
    base_url: &str,
    model: &str,
    framework: &str,
    domain: &str,
    research_questions: &[&str],
    dimension: &str,
    dimension_hint: &str,
    existing_codes: &[String],
    title: &str,
    authors: Option<&str>,
    abstract_text: Option<&str>,
    api_key: Option<&str>,
) -> Result<SuggestResult> {
    let codes_block = if existing_codes.is_empty() {
        "(no codes have been used in this dimension yet — propose new ones).".to_string()
    } else {
        existing_codes
            .iter()
            .map(|c| format!("- {c}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let rqs_block = research_questions
        .iter()
        .enumerate()
        .map(|(i, q)| format!("{}. {q}", i + 1))
        .collect::<Vec<_>>()
        .join("\n");
    let abstract_block = abstract_text.unwrap_or("(no abstract available)");
    let authors_block = authors.unwrap_or("(authors not recorded)");

    let system = format!(
        "You are a meticulous research assistant helping a scholar code articles for a systematic literature review. \
You suggest codes — you do not make decisions. The researcher always has final say.

Review domain: {domain}
Organising framework: {framework}
Research questions:
{rqs_block}

You will be asked to suggest codes for ONE dimension of the framework at a time."
    );

    let user = format!(
        "Dimension to code: {dimension}
Dimension means: {dimension_hint}

Codes already used in this dimension across the corpus (reuse when accurate):
{codes_block}

Article to code:
Title: {title}
Authors: {authors_block}
Abstract: {abstract_block}

Suggest 1 to 3 codes for the {dimension} dimension. Reuse an existing code when it fits; only mark a code is_new=true when no existing code applies. Each code should be a short noun phrase (2-5 words), not a sentence.

Respond ONLY with a JSON array of objects, each with keys: code (string), rationale (string, one short sentence), is_new (boolean). No prose before or after."
    );

    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];

    let result = lmstudio::chat(base_url, model, &messages, Some(0.2), api_key).await?;

    let suggestions = parse_suggestions(&result.content)
        .ok_or_else(|| anyhow!("could not parse JSON suggestions from model output"))?;

    Ok(SuggestResult {
        suggestions,
        model: result.model,
        latency_ms: result.latency_ms,
        raw: result.content,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn suggest_query(
    base_url: &str,
    model: &str,
    domain: &str,
    framework: &str,
    rq_what: Option<&str>,
    rq_how: Option<&str>,
    rq_where: Option<&str>,
    selected_databases: &[String],
    existing_query: Option<&str>,
    api_key: Option<&str>,
) -> Result<SuggestQueryResult> {
    let db_list = if selected_databases.is_empty() {
        "(no databases selected yet)".to_string()
    } else {
        selected_databases
            .iter()
            .map(|d| format!("- {d}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let rqs_block = [
        rq_what.map(|q| format!("What: {q}")),
        rq_how.map(|q| format!("How: {q}")),
        rq_where.map(|q| format!("Where: {q}")),
    ]
    .iter()
    .filter_map(|x| x.as_ref())
    .cloned()
    .collect::<Vec<_>>()
    .join("\n");
    let rqs_block = if rqs_block.is_empty() {
        "(not specified)".to_string()
    } else {
        rqs_block
    };

    let system = format!(
        "You are a systematic literature review expert helping a researcher build a boolean search query. \
Your queries use AND between concepts, OR between synonyms, and wildcards (*) where appropriate. \
You return ONLY valid JSON — no prose before or after.

Review domain: {domain}
Organising framework: {framework}
Research questions:
{rqs_block}

Target databases:
{db_list}"
    );

    let user = match existing_query {
        None => "Generate a boolean search query for this systematic literature review. \
Return a JSON object with exactly two keys: \
\"query\" (the full boolean search string, ready to paste into a database) and \
\"rationale\" (one sentence explaining the key choices made). \
No prose outside the JSON object.".to_string(),

        Some(q) => format!(
            "Here is the researcher's current boolean query:\n\n{q}\n\n\
Advise and improve it: fix any weaknesses, add missing synonyms, improve structure. \
Return a JSON object with exactly two keys: \
\"query\" (the improved boolean search string) and \
\"rationale\" (one sentence explaining what was changed and why). \
No prose outside the JSON object."
        ),
    };

    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];

    let result = lmstudio::chat(base_url, model, &messages, Some(0.3), api_key).await?;

    let (query, rationale) = parse_query_result(&result.content)
        .ok_or_else(|| anyhow!("could not parse JSON query result from model output"))?;

    Ok(SuggestQueryResult {
        query,
        rationale,
        model: result.model,
        latency_ms: result.latency_ms,
    })
}

/// Extract `query` and `rationale` from a JSON object in the model's response.
/// Tolerates prose before/after the object.
fn parse_query_result(s: &str) -> Option<(String, String)> {
    let bytes = s.as_bytes();
    let start = bytes.iter().position(|&b| b == b'{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut end = None;
    for i in start..bytes.len() {
        let c = bytes[i];
        if esc {
            esc = false;
            continue;
        }
        if c == b'\\' {
            esc = true;
            continue;
        }
        if c == b'"' {
            in_str = !in_str;
            continue;
        }
        if in_str {
            continue;
        }
        if c == b'{' { depth += 1; }
        else if c == b'}' {
            depth -= 1;
            if depth == 0 { end = Some(i + 1); break; }
        }
    }
    let end = end?;
    let slice = &s[start..end];
    let v: serde_json::Value = serde_json::from_str(slice).ok()?;
    let query = v.get("query")?.as_str()?.to_string();
    let rationale = v.get("rationale")?.as_str()?.to_string();
    Some((query, rationale))
}

/// Find the first JSON array in the model's response and parse it as a list
/// of Suggestion objects. Tolerant: skips any prose before/after.
fn parse_suggestions(s: &str) -> Option<Vec<Suggestion>> {
    let bytes = s.as_bytes();
    let start = bytes.iter().position(|&b| b == b'[')?;
    // Find matching closing bracket (simple depth tracking, ignores brackets in strings — good enough for our model output)
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut end = None;
    for i in start..bytes.len() {
        let c = bytes[i];
        if esc {
            esc = false;
            continue;
        }
        if c == b'\\' {
            esc = true;
            continue;
        }
        if c == b'"' {
            in_str = !in_str;
            continue;
        }
        if in_str {
            continue;
        }
        if c == b'[' {
            depth += 1;
        } else if c == b']' {
            depth -= 1;
            if depth == 0 {
                end = Some(i + 1);
                break;
            }
        }
    }
    let end = end?;
    let slice = &s[start..end];
    serde_json::from_str::<Vec<Suggestion>>(slice).ok()
}
