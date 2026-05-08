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
    #[serde(alias = "codes", alias = "name", alias = "label")]
    pub code: String,
    #[serde(default, alias = "rationales", alias = "reason", alias = "reasoning", alias = "explanation")]
    pub rationale: Option<String>,
    #[serde(default, alias = "isNew", alias = "new")]
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
pub struct StepQuestion {
    pub q: String,
    /// 0–4 options. Empty means the UI should render a free-text input only.
    pub options: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StepQuestionsResult {
    pub questions: Vec<StepQuestion>,
    pub model: Option<String>,
    pub latency_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QAndA {
    pub q: String,
    pub a: String,
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

/// Generate 1–3 multi-choice clarifying questions for an onboarding step.
/// The model is told the step kind and the researcher's current draft, then
/// asked to surface the most useful disambiguating questions before any rewrite.
#[allow(clippy::too_many_arguments)]
pub async fn suggest_step_questions(
    base_url: &str,
    model: &str,
    step_id: &str,
    current_value: &str,
    project_name: &str,
    slr_type: &str,
    framework: &str,
    domain: &str,
    api_key: Option<&str>,
) -> Result<StepQuestionsResult> {
    let system = "You are a systematic literature review expert assisting a researcher \
through the SPAR-4-SLR onboarding process. \
Before suggesting any rewrite, you ask 1–3 sharp clarifying questions that help \
disambiguate the researcher's intent — about scope, audience, methods, or boundaries. \
Each question MUST have either 2–4 plausible discrete options OR an empty options array \
when the answer space is genuinely open-ended (the UI will render a free-text input). \
Respond ONLY with a JSON object of shape: \
{\"questions\":[{\"q\":\"...\",\"options\":[\"...\"]}]}. \
No prose outside the JSON.".to_string();

    let context = match step_id {
        "name" => format!(
            "Step: working title for the review.\n\
Current draft: \"{current_value}\"\n\
Ask questions that help pin down: domain term to use, scope/period, intended audience, contribution angle."
        ),
        "slr_type" => format!(
            "Step: SLR type selection.\n\
Project title: {project_name}\n\
Chosen type: \"{current_value}\"\n\
Ask questions that help confirm whether this type fits the researcher's contribution goals \
(domain map vs theory trace vs method survey vs effect-size pooling vs review-of-reviews)."
        ),
        "framework" => format!(
            "Step: organising framework selection.\n\
Project title: {project_name}\n\
SLR type: {slr_type}\n\
Domain: {domain}\n\
Chosen framework: \"{current_value}\"\n\
Ask questions about coding granularity, theoretical maturity of the field, and time-budget tolerance \
that help confirm or challenge this framework choice."
        ),
        "domain" => format!(
            "Step: domain description (paragraph).\n\
Project title: {project_name}\n\
SLR type: {slr_type}\n\
Framework: {framework}\n\
Current draft:\n\"\"\"\n{current_value}\n\"\"\"\n\
Ask questions that surface the in-scope/out-of-scope boundary, ambiguous terms, and edge cases \
the researcher may not have explicitly addressed."
        ),
        "rq_what" => format!(
            "Step: 'What do we know?' research question.\n\
Project title: {project_name}\n\
Domain: {domain}\n\
Framework: {framework}\n\
Current draft:\n\"\"\"\n{current_value}\n\"\"\"\n\
Ask questions that help calibrate breadth (whole-corpus vs subset), unit of analysis, and the kind of \
findings the researcher cares about (concepts, debates, effects)."
        ),
        "rq_how" => format!(
            "Step: 'How do we know?' research question.\n\
Project title: {project_name}\n\
Domain: {domain}\n\
Framework: {framework}\n\
Current draft:\n\"\"\"\n{current_value}\n\"\"\"\n\
Ask questions that help focus methodological lenses — theories, designs, samples, measurement — \
and identify the methodological gap the researcher most wants to surface."
        ),
        "rq_where" => format!(
            "Step: 'Where should the field head?' research question.\n\
Project title: {project_name}\n\
Domain: {domain}\n\
Framework: {framework}\n\
Current draft:\n\"\"\"\n{current_value}\n\"\"\"\n\
Ask questions that help identify which kind of agenda the researcher wants to set: \
unexplored contexts, contradictions to resolve, methodological extensions, or theoretical bridges."
        ),
        other => return Err(anyhow!("unknown step_id: {other}")),
    };

    let user = format!("{context}\n\nReturn the JSON object now.");

    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];

    let result = lmstudio::chat(base_url, model, &messages, Some(0.4), api_key).await?;

    let questions = parse_step_questions(&result.content)
        .ok_or_else(|| anyhow!("could not parse clarifying questions from model output"))?;

    if questions.is_empty() {
        return Err(anyhow!("model returned no clarifying questions"));
    }

    Ok(StepQuestionsResult {
        questions,
        model: result.model,
        latency_ms: result.latency_ms,
    })
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
    answers: Option<&[QAndA]>,
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

    let user = if let Some(qa) = answers.filter(|a| !a.is_empty()) {
        let block = qa
            .iter()
            .map(|p| format!("Q: {}\nA: {}", p.q, p.a))
            .collect::<Vec<_>>()
            .join("\n\n");
        format!(
            "Researcher's clarifications:\n{block}\n\nUse these clarifications to inform the rewrite below.\n\n{user}"
        )
    } else {
        user
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
    let suggestion = pick_str(
        &v,
        &["suggestion", "suggestions", "improved", "revised", "result", "output", "text"],
    )
    .unwrap_or("")
    .to_string();
    let rationale = pick_str(
        &v,
        &["rationale", "rationales", "reason", "reasoning", "explanation", "notes"],
    )
    .unwrap_or("")
    .to_string();
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

    let inclusion_list = inclusion_criteria
        .iter()
        .map(|c| format!("- {c}"))
        .collect::<Vec<_>>()
        .join("\n");
    let exclusion_list = exclusion_criteria
        .iter()
        .map(|c| format!("- {c}"))
        .collect::<Vec<_>>()
        .join("\n");

    // Criteria block: visible in the user message so the model can't overlook it.
    // Empty when neither list has content, so the prompt stays clean for projects
    // that haven't recorded eligibility criteria yet.
    let criteria_section = match (inclusion_criteria.is_empty(), exclusion_criteria.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!(
            "\nEligibility criteria the researcher has recorded:\n\nInclusion (records must satisfy):\n{inclusion_list}\n\n\
Where these criteria reference subject-matter concepts (topics, populations, contexts, methods named in the criterion), \
fold matching keyword groups into the query so the search itself targets eligible records. \
Where they reference publication metadata only (study type, language, year range, peer-reviewed status), \
note in your rationale that screening will enforce them and do not force them into the boolean string.\n"
        ),
        (true, false) => format!(
            "\nEligibility criteria the researcher has recorded:\n\nExclusion (records that match should be omitted):\n{exclusion_list}\n\n\
Where these criteria reference subject-matter concepts, avoid keyword groups that would invite those records, \
or use boolean NOT clauses if the unwanted concept has a precise term. \
Where they reference publication metadata only, note in your rationale that screening will enforce them and do not force them into the boolean string.\n"
        ),
        (false, false) => format!(
            "\nEligibility criteria the researcher has recorded:\n\nInclusion (records must satisfy):\n{inclusion_list}\n\nExclusion (records that match should be omitted):\n{exclusion_list}\n\n\
Where these criteria reference subject-matter concepts (topics, populations, contexts, methods named in the criterion), \
weave matching keyword groups into the query so the search itself targets eligible records. \
For exclusion criteria with a precise term, you may use boolean NOT clauses. \
Where criteria reference publication metadata only (study type, language, year range, peer-reviewed status), \
note in your rationale that screening will enforce them and do not force them into the boolean string.\n"
        ),
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
        None => format!(
            "Generate a boolean search query for this systematic literature review.\n\
{criteria_section}\n\
Return a JSON object with exactly two keys:\n\
- \"query\": the full boolean search string, ready to paste into a database\n\
- \"rationale\": one sentence explaining the key term choices, including which inclusion or exclusion criteria informed your keyword selection (or why they were left for screening)\n\
No prose outside the JSON object."
        ),

        Some(q) => format!(
            "Here is the researcher's current boolean query:\n\n{q}\n\n\
{criteria_section}\n\
Advise and improve it: fix any weaknesses, add missing synonyms or concept groups (especially ones suggested by the inclusion criteria), and trim or NOT-out concepts the exclusion criteria would reject. \
Return a JSON object with exactly two keys:\n\
- \"query\": the improved boolean search string\n\
- \"rationale\": one sentence explaining what was changed and why, naming any criterion that drove a change\n\
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

/// Extract a list of clarifying questions from a model's JSON response.
/// Tolerates prose before/after the object, markdown fences, and key drift
/// (`questions`/`question`, `q`/`question`/`prompt`, `options`/`choices`).
/// Entries missing the question text are silently dropped.
fn parse_step_questions(s: &str) -> Option<Vec<StepQuestion>> {
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

    let arr = v.get("questions")
        .or_else(|| v.get("question"))
        .or_else(|| v.get("items"))
        .and_then(|x| x.as_array())?;

    let mut out = Vec::with_capacity(arr.len());
    for entry in arr {
        let q = pick_str(entry, &["q", "question", "prompt", "text"]);
        let q = match q {
            Some(s) if !s.trim().is_empty() => s.to_string(),
            _ => continue,
        };
        let options = entry
            .get("options")
            .or_else(|| entry.get("choices"))
            .or_else(|| entry.get("answers"))
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|o| o.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        out.push(StepQuestion { q, options });
    }
    Some(out)
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

    #[test]
    fn suggestion_deserializes_canonical_keys() {
        let s = r#"{"code":"X","rationale":"y","is_new":true}"#;
        let sug: Suggestion = serde_json::from_str(s).unwrap();
        assert_eq!(sug.code, "X");
        assert_eq!(sug.rationale.as_deref(), Some("y"));
        assert!(sug.is_new);
    }

    #[test]
    fn suggestion_deserializes_alias_keys() {
        // codes (plural), rationales (plural), isNew (camelCase) all accepted.
        let s = r#"{"codes":"X","rationales":"y","isNew":true}"#;
        let sug: Suggestion = serde_json::from_str(s).unwrap();
        assert_eq!(sug.code, "X");
        assert_eq!(sug.rationale.as_deref(), Some("y"));
        assert!(sug.is_new);
    }

    #[test]
    fn suggestion_array_parses_with_alias_keys() {
        // parse_suggestions tolerates per-element key drift.
        let s = r#"[
            {"code":"A","rationale":"first"},
            {"codes":"B","reason":"second","new":true}
        ]"#;
        let v = parse_suggestions(s).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].code, "A");
        assert_eq!(v[0].rationale.as_deref(), Some("first"));
        assert_eq!(v[1].code, "B");
        assert_eq!(v[1].rationale.as_deref(), Some("second"));
        assert!(v[1].is_new);
    }

    #[test]
    fn parse_step_questions_canonical() {
        let s = r#"{
            "questions": [
                {"q": "What is the audience?", "options": ["Practitioners", "Academics"]},
                {"q": "Pick a region", "options": ["EU", "US", "Global"]}
            ]
        }"#;
        let qs = parse_step_questions(s).unwrap();
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].q, "What is the audience?");
        assert_eq!(qs[0].options, vec!["Practitioners", "Academics"]);
    }

    #[test]
    fn parse_step_questions_empty_options_for_freetext() {
        let s = r#"{"questions": [{"q": "Describe in your words", "options": []}]}"#;
        let qs = parse_step_questions(s).unwrap();
        assert_eq!(qs.len(), 1);
        assert!(qs[0].options.is_empty());
    }

    #[test]
    fn parse_step_questions_alias_keys() {
        // question (singular) instead of questions; "question" / "prompt" instead of "q";
        // "choices" instead of "options".
        let s = r#"{"question": [{"prompt": "Pick scope", "choices": ["narrow", "broad"]}]}"#;
        let qs = parse_step_questions(s).unwrap();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q, "Pick scope");
        assert_eq!(qs[0].options, vec!["narrow", "broad"]);
    }

    #[test]
    fn parse_step_questions_drops_malformed_entries() {
        // entries without q/question are skipped; the rest still parse.
        let s = r#"{"questions": [
            {"q": "ok", "options": ["a"]},
            {"options": ["no q"]},
            {"q": "also ok", "options": []}
        ]}"#;
        let qs = parse_step_questions(s).unwrap();
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].q, "ok");
        assert_eq!(qs[1].q, "also ok");
    }

    #[test]
    fn parse_step_questions_fails_when_no_object() {
        assert!(parse_step_questions("not json at all").is_none());
    }

    #[test]
    fn parse_step_questions_markdown_fenced() {
        let s = "```json\n{\"questions\":[{\"q\":\"A?\",\"options\":[\"x\",\"y\"]}]}\n```";
        let qs = parse_step_questions(s).unwrap();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q, "A?");
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
/// `profile` is prepended to the system prompt when non-empty, shaping how the rater evaluates.
/// `journal`, `year`, and `authors` are passed alongside title + abstract so the model can
/// evaluate criteria that depend on publication metadata (predatory journal, source type,
/// search period sanity check, etc.).
#[allow(clippy::too_many_arguments)]
pub async fn screen_article(
    base_url: &str,
    model: &str,
    title: &str,
    abstract_text: Option<&str>,
    journal: Option<&str>,
    year: Option<i32>,
    authors: Option<&str>,
    domain: &str,
    exclusion_criteria_labeled: &[(i64, String)], // (criterion_number, label)
    profile: &str,
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

    let profile_block = if profile.trim().is_empty() {
        String::new()
    } else {
        format!(
            "\n\nRater profile (read carefully — this shapes how YOU evaluate this article):\n{}\n",
            profile.trim()
        )
    };

    let system = format!(
        "You are a systematic literature review screener applying title-and-abstract level screening, \
         with the article's publication metadata available alongside the abstract.{profile_block}\n\
         You decide whether each article should be included, excluded, or flagged as uncertain.\n\n\
         Domain scope of this review:\n{domain_block}\n\n\
         Exclusion criteria (exclude the article if it meets ANY of these):\n{exc_block}\n\n\
         When a criterion concerns publication metadata (journal name, publication year, authors), \
         use the metadata fields below — not just the abstract — to evaluate it. When a criterion \
         is about content or scope, read the abstract.\n\n\
         Respond ONLY with a JSON object with exactly four keys:\n\
         - \"decision\": \"include\" if the article is relevant to the domain scope and passes all exclusion criteria, \
         \"exclude\" if it meets any exclusion criterion, or \"uncertain\" if relevance is unclear\n\
         - \"criterion_number\": the integer EC number if decision is \"exclude\", otherwise null\n\
         - \"rationale\": one sentence explaining the decision\n\
         - \"confidence\": a number 0.0–1.0 (how confident you are)\n\
         No prose outside the JSON."
    );

    let abstract_block = abstract_text.unwrap_or("(no abstract available)");
    let journal_block = journal
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("(not recorded)");
    let year_block = year
        .map(|y| y.to_string())
        .unwrap_or_else(|| "(not recorded)".to_string());
    let authors_block = authors
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("(not recorded)");
    let user = format!(
        "Title: {title}\n\
         Authors: {authors_block}\n\
         Journal: {journal_block}\n\
         Year: {year_block}\n\
         Abstract: {abstract_block}\n\n\
         Return the JSON decision."
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
    profile: &str,
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

    let coding_profile_block = if profile.trim().is_empty() {
        String::new()
    } else {
        format!(
            "\n\nRater profile (read carefully — this shapes how YOU evaluate this article):\n{}\n",
            profile.trim()
        )
    };

    let system = format!(
        "You are a meticulous research assistant helping a scholar code articles for a systematic \
literature review. You suggest codes — you do not make decisions. The researcher always has final say.{coding_profile_block}\n\n\
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
