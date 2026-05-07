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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DimensionCoding {
    pub dimension: String,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScreeningDecision {
    pub decision: String,         // "include" | "exclude" | "uncertain"
    pub criterion_number: Option<i64>,
    pub rationale: String,
    pub confidence: f64,
}

/// Generic step assistant for all 7 onboarding steps.
/// Returns (suggestion, rationale). For text steps the suggestion is a refined
/// replacement; for choice steps it is advisory/validation text.
#[allow(clippy::too_many_arguments)]
pub async fn suggest_step(
    base_url: &str,
    model: &str,
    step_id: &str,
    current_value: &str,
    project_name: &str,
    slr_type: &str,
    framework: &str,
    domain: &str,
    api_key: Option<&str>,
) -> Result<(String, String)> {
    let system = "You are a systematic literature review expert assisting a researcher \
through the SPAR-4-SLR onboarding process. \
Respond ONLY with a JSON object containing exactly two keys: \
\"suggestion\" (your refined text or advice, plain prose) and \
\"rationale\" (one sentence explaining the key choice you made). \
No prose outside the JSON.".to_string();

    let user = match step_id {
        "name" => format!(
            "The researcher is naming their systematic literature review.\n\
Current working title: \"{current_value}\"\n\n\
Suggest a clearer, more precise working title that:\n\
- Signals the domain and scope immediately\n\
- Is concise (ideally under 12 words)\n\
- Avoids vague words like 'study of' or 'investigation into'\n\
Return the JSON."
        ),

        "slr_type" => format!(
            "Review title: {project_name}\n\
The researcher has chosen SLR type: \"{current_value}\"\n\n\
Evaluate whether this is the right type for their review. \
Explain in 2–3 sentences why this type fits (or flag if another type would be more appropriate). \
Be specific to the title and domain implied by the title.\n\
Return the JSON (suggestion = your evaluation paragraph)."
        ),

        "framework" => format!(
            "Review title: {project_name}\n\
SLR type: {slr_type}\n\
Domain: {domain}\n\
The researcher has chosen framework: \"{current_value}\"\n\n\
Evaluate whether this framework fits their SLR type and domain in 2–3 sentences. \
If it's a strong fit, explain why. If another framework would be better, say which and why.\n\
Return the JSON (suggestion = your evaluation paragraph)."
        ),

        "domain" => format!(
            "Review title: {project_name}\n\
SLR type: {slr_type}\n\
Framework: {framework}\n\n\
Current domain description:\n\"\"\"\n{current_value}\n\"\"\"\n\n\
Improve this description so that it:\n\
- Is concrete about what is in scope and what is explicitly out\n\
- Defines any ambiguous boundary terms\n\
- Reads as a focused paragraph (2–5 sentences), not a list\n\
Preserve the researcher's intent; only sharpen it.\n\
Return the JSON (suggestion = the improved paragraph)."
        ),

        "rq_what" => format!(
            "Review title: {project_name}\n\
SLR type: {slr_type}\n\
Framework: {framework}\n\
Domain: {domain}\n\n\
Current 'What do we know?' research question:\n\"\"\"\n{current_value}\n\"\"\"\n\n\
Refine this question so that it:\n\
- Is phrased as a clear, answerable question\n\
- Is broad enough for the whole corpus to contribute\n\
- Aligns with the domain and framework\n\
Return the JSON (suggestion = the refined question)."
        ),

        "rq_how" => format!(
            "Review title: {project_name}\n\
SLR type: {slr_type}\n\
Framework: {framework}\n\
Domain: {domain}\n\n\
Current 'How do we know?' research question:\n\"\"\"\n{current_value}\n\"\"\"\n\n\
Refine this question so that it:\n\
- Addresses theories, methods, and contexts used in the domain\n\
- Is phrased as a clear, answerable question\n\
- Surfaces methodological or theoretical gaps\n\
Return the JSON (suggestion = the refined question)."
        ),

        "rq_where" => format!(
            "Review title: {project_name}\n\
SLR type: {slr_type}\n\
Framework: {framework}\n\
Domain: {domain}\n\n\
Current 'Where should the field head?' research question:\n\"\"\"\n{current_value}\n\"\"\"\n\n\
Refine this question so that it:\n\
- Points clearly at the future research agenda\n\
- Highlights gaps, contradictions, or unexplored contexts\n\
- Is phrased as a clear, answerable question\n\
Return the JSON (suggestion = the refined question)."
        ),

        other => return Err(anyhow!("unknown step_id: {other}")),
    };

    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];

    let result = lmstudio::chat(base_url, model, &messages, Some(0.4), api_key).await?;

    // Extract the first JSON object from the response
    let bytes = result.content.as_bytes();
    let start = bytes.iter().position(|&b| b == b'{')
        .ok_or_else(|| anyhow!("no JSON object in model response"))?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut end = None;
    for i in start..bytes.len() {
        let c = bytes[i];
        if esc { esc = false; continue; }
        if c == b'\\' { esc = true; continue; }
        if c == b'"' { in_str = !in_str; continue; }
        if in_str { continue; }
        if c == b'{' { depth += 1; }
        else if c == b'}' { depth -= 1; if depth == 0 { end = Some(i + 1); break; } }
    }
    let end = end.ok_or_else(|| anyhow!("unterminated JSON object"))?;
    let v: serde_json::Value = serde_json::from_str(&result.content[start..end])?;
    let suggestion = v.get("suggestion").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let rationale = v.get("rationale").and_then(|x| x.as_str()).unwrap_or("").to_string();
    Ok((suggestion, rationale))
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
    inclusion_criteria: &[String],
    exclusion_criteria: &[String],
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

    let inclusion_block = if inclusion_criteria.is_empty() {
        String::new()
    } else {
        let list = inclusion_criteria
            .iter()
            .map(|c| format!("- {c}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("\nSearch inclusion criteria (records must satisfy):\n{list}")
    };

    let exclusion_block = if exclusion_criteria.is_empty() {
        String::new()
    } else {
        let list = exclusion_criteria
            .iter()
            .map(|c| format!("- {c}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("\nSearch exclusion criteria (records that match should be omitted):\n{list}")
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
{db_list}{inclusion_block}{exclusion_block}"
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
    let query = pick_str(&v, &["query", "queries", "boolean_query", "search_query"])?
        .to_string();
    let rationale = pick_str(
        &v,
        &["rationale", "rationales", "reason", "reasoning", "explanation", "notes"],
    )
    .unwrap_or("")
    .to_string();
    Some((query, rationale))
}

fn pick_str<'a>(v: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            return Some(s);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_query_result_canonical_keys() {
        let s = r#"{"query": "X AND Y", "rationale": "because"}"#;
        let (q, r) = parse_query_result(s).unwrap();
        assert_eq!(q, "X AND Y");
        assert_eq!(r, "because");
    }

    #[test]
    fn parse_query_result_plural_rationales() {
        // gemma-4-26b-a4b and others sometimes emit "rationales" (plural).
        let s = r#"{"query": "X AND Y", "rationales": "because"}"#;
        let (q, r) = parse_query_result(s).unwrap();
        assert_eq!(q, "X AND Y");
        assert_eq!(r, "because");
    }

    #[test]
    fn parse_query_result_missing_rationale_is_empty() {
        let s = r#"{"query": "X AND Y"}"#;
        let (q, r) = parse_query_result(s).unwrap();
        assert_eq!(q, "X AND Y");
        assert_eq!(r, "");
    }

    #[test]
    fn parse_query_result_markdown_fenced() {
        let s = "```json\n{\"query\": \"X AND Y\", \"rationale\": \"because\"}\n```";
        let (q, r) = parse_query_result(s).unwrap();
        assert_eq!(q, "X AND Y");
        assert_eq!(r, "because");
    }

    #[test]
    fn parse_query_result_fails_without_query() {
        let s = r#"{"rationale": "no query here"}"#;
        assert!(parse_query_result(s).is_none());
    }

    #[test]
    fn parse_screening_decision_canonical() {
        let s = r#"{"decision":"include","rationale":"on topic","confidence":0.9}"#;
        let d = parse_screening_decision(s).unwrap();
        assert_eq!(d.decision, "include");
        assert_eq!(d.rationale, "on topic");
        assert!((d.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(d.criterion_number, None);
    }

    #[test]
    fn parse_screening_decision_plural_rationales() {
        // Same model fragility as boolean query — rationale → rationales.
        let s = r#"{"decision":"exclude","criterion_number":2,"rationales":"off topic","confidence":0.85}"#;
        let d = parse_screening_decision(s).unwrap();
        assert_eq!(d.decision, "exclude");
        assert_eq!(d.criterion_number, Some(2));
        assert_eq!(d.rationale, "off topic");
    }

    #[test]
    fn parse_screening_decision_missing_rationale_is_empty() {
        let s = r#"{"decision":"uncertain","confidence":0.5}"#;
        let d = parse_screening_decision(s).unwrap();
        assert_eq!(d.decision, "uncertain");
        assert_eq!(d.rationale, "");
    }

    #[test]
    fn parse_screening_decision_rejects_unknown_decision() {
        let s = r#"{"decision":"maybe","rationale":"x","confidence":0.5}"#;
        assert!(parse_screening_decision(s).is_none());
    }

    #[test]
    fn parse_screening_decision_alternate_keys() {
        // verdict instead of decision, score instead of confidence.
        let s = r#"{"verdict":"include","reason":"matches scope","score":0.7}"#;
        let d = parse_screening_decision(s).unwrap();
        assert_eq!(d.decision, "include");
        assert_eq!(d.rationale, "matches scope");
        assert!((d.confidence - 0.7).abs() < f64::EPSILON);
    }
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

/// Screen a single article against the review's domain scope and exclusion criteria.
/// Returns a ScreeningDecision with decision, optional criterion, rationale, and confidence.
pub async fn screen_article(
    base_url: &str,
    model: &str,
    title: &str,
    abstract_text: Option<&str>,
    domain: &str,
    exclusion_criteria_labeled: &[(i64, String)], // (criterion_number, label)
    api_key: Option<&str>,
) -> Result<ScreeningDecision> {
    let domain_block = if domain.trim().is_empty() {
        "(not specified)".to_string()
    } else {
        domain.to_string()
    };

    let exc_block = if exclusion_criteria_labeled.is_empty() {
        "(none specified)".to_string()
    } else {
        exclusion_criteria_labeled
            .iter()
            .map(|(n, l)| format!("EC{n}: {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let system = format!(
        "You are a systematic literature review screener applying title and abstract screening.\n\
         You decide whether each article should be included, excluded, or flagged as uncertain.\n\n\
         Domain scope of this review:\n{domain_block}\n\n\
         Exclusion criteria (exclude the article if it meets ANY of these):\n{exc_block}\n\n\
         Respond ONLY with a JSON object with exactly four keys:\n\
         - \"decision\": \"include\" if the article is relevant to the domain scope and passes all exclusion criteria, \
         \"exclude\" if it meets any exclusion criterion, or \"uncertain\" if relevance is unclear\n\
         - \"criterion_number\": the integer EC number if decision is \"exclude\", otherwise null\n\
         - \"rationale\": one sentence explaining the decision\n\
         - \"confidence\": a number 0.0–1.0 (how confident you are)\n\
         No prose outside the JSON."
    );

    let abstract_block = abstract_text.unwrap_or("(no abstract available)");
    let user = format!(
        "Title: {title}\nAbstract: {abstract_block}\n\nReturn the JSON decision."
    );

    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];

    let result = lmstudio::chat(base_url, model, &messages, Some(0.1), api_key).await?;

    parse_screening_decision(&result.content)
        .ok_or_else(|| anyhow::anyhow!("could not parse screening decision from model output: {}", result.content))
}

fn parse_screening_decision(s: &str) -> Option<ScreeningDecision> {
    let bytes = s.as_bytes();
    let start = bytes.iter().position(|&b| b == b'{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut end = None;
    for i in start..bytes.len() {
        let c = bytes[i];
        if esc { esc = false; continue; }
        if c == b'\\' { esc = true; continue; }
        if c == b'"' { in_str = !in_str; continue; }
        if in_str { continue; }
        if c == b'{' { depth += 1; }
        else if c == b'}' { depth -= 1; if depth == 0 { end = Some(i + 1); break; } }
    }
    let end = end?;
    let v: serde_json::Value = serde_json::from_str(&s[start..end]).ok()?;
    let decision = pick_str(&v, &["decision", "verdict", "result"])?.to_string();
    if !["include", "exclude", "uncertain"].contains(&decision.as_str()) {
        return None;
    }
    let criterion_number = if decision == "exclude" {
        v.get("criterion_number")
            .or_else(|| v.get("criterion"))
            .or_else(|| v.get("ec"))
            .or_else(|| v.get("ec_number"))
            .and_then(|x| x.as_i64())
    } else {
        None
    };
    let rationale = pick_str(
        &v,
        &["rationale", "rationales", "reason", "reasoning", "explanation", "notes"],
    )
    .unwrap_or("")
    .to_string();
    let confidence = ["confidence", "confidence_score", "score"]
        .iter()
        .find_map(|k| v.get(*k).and_then(|x| x.as_f64()))?;
    Some(ScreeningDecision { decision, criterion_number, rationale, confidence })
}

/// Code a single article across multiple dimensions in one LLM call.
/// `context_text` is the concatenated first N chunks of the article's PDF.
/// Returns one `DimensionCoding` per requested dimension (may have empty suggestions
/// if the model skipped it).
#[allow(clippy::too_many_arguments)]
pub async fn code_article_for_dimensions(
    base_url: &str,
    model: &str,
    framework: &str,
    domain: &str,
    research_questions: &[&str],
    dimensions: &[(String, String)], // (name, description)
    existing_codes_by_dim: &std::collections::HashMap<String, Vec<String>>,
    title: &str,
    authors: Option<&str>,
    context_text: &str,
    api_key: Option<&str>,
) -> Result<Vec<DimensionCoding>> {
    let rqs_block = if research_questions.is_empty() {
        "(not specified)".to_string()
    } else {
        research_questions
            .iter()
            .enumerate()
            .map(|(i, q)| format!("{}. {q}", i + 1))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let dims_block = dimensions
        .iter()
        .map(|(name, desc)| {
            let existing = existing_codes_by_dim.get(name.as_str())
                .filter(|v| !v.is_empty())
                .map(|v| v.iter().map(|c| format!("  - {c}")).collect::<Vec<_>>().join("\n"))
                .unwrap_or_else(|| "  (none yet — propose new ones)".to_string());
            format!("Dimension \"{name}\": {desc}\n  Existing codes:\n{existing}")
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let dim_names: Vec<&str> = dimensions.iter().map(|(n, _)| n.as_str()).collect();
    let dim_list = dim_names.join(", ");

    let context_block = if context_text.trim().is_empty() {
        "(no PDF text available)".to_string()
    } else {
        // Truncate to keep the prompt manageable (~8 000 chars)
        let truncated = if context_text.len() > 8_000 {
            &context_text[..8_000]
        } else {
            context_text
        };
        truncated.to_string()
    };

    let system = format!(
        "You are a meticulous research assistant helping a scholar code articles for a systematic \
literature review. You suggest codes — you do not make decisions. The researcher always has final say.\n\n\
Review domain: {domain}\nOrganising framework: {framework}\nResearch questions:\n{rqs_block}\n\n\
For each dimension you will suggest 1–3 short noun-phrase codes (2–5 words each). \
Reuse an existing code when it fits; only mark is_new=true when no existing code is appropriate.\n\n\
Respond ONLY with a JSON object. The keys are the exact dimension names provided. \
Each value is a JSON array of objects with keys: code (string), rationale (string, one sentence), is_new (boolean).\n\
No prose before or after the JSON."
    );

    let user = format!(
        "Dimensions to code: {dim_list}\n\n{dims_block}\n\n\
Article to code:\nTitle: {title}\nAuthors: {}\n\nFull text excerpt:\n{context_block}\n\n\
Return the JSON object now.",
        authors.unwrap_or("(not recorded)")
    );

    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];

    let result = lmstudio::chat(base_url, model, &messages, Some(0.2), api_key).await?;

    let codings = parse_dimension_codings(&result.content, &dim_names)
        .ok_or_else(|| anyhow::anyhow!("could not parse dimension codings from: {}", &result.content))?;

    Ok(codings)
}

fn parse_dimension_codings(s: &str, dimensions: &[&str]) -> Option<Vec<DimensionCoding>> {
    let bytes = s.as_bytes();
    let start = bytes.iter().position(|&b| b == b'{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut end = None;
    for i in start..bytes.len() {
        let c = bytes[i];
        if esc { esc = false; continue; }
        if c == b'\\' { esc = true; continue; }
        if c == b'"' { in_str = !in_str; continue; }
        if in_str { continue; }
        if c == b'{' { depth += 1; }
        else if c == b'}' {
            depth -= 1;
            if depth == 0 { end = Some(i + 1); break; }
        }
    }
    let end = end?;
    let v: serde_json::Value = serde_json::from_str(&s[start..end]).ok()?;
    let obj = v.as_object()?;
    let mut result = Vec::new();
    for dim in dimensions {
        let suggestions = obj
            .get(*dim)
            .and_then(|arr| serde_json::from_value::<Vec<Suggestion>>(arr.clone()).ok())
            .unwrap_or_default();
        result.push(DimensionCoding {
            dimension: dim.to_string(),
            suggestions,
        });
    }
    Some(result)
}
