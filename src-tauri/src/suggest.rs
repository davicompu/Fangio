//! Local-model code suggestion for the arranging stage.
//!
//! Builds a focused, ≤80-line prompt and calls LM Studio. The model is asked
//! to return JSON; we tolerate prose around the JSON and extract the first
//! array we find. Reasoning traces are stripped upstream by the LM Studio
//! client.

use crate::lmstudio::{self, ChatMessage};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Provenance of one AI call: which model actually answered, a fingerprint of
/// the system prompt, and the sampling settings used.
#[derive(Debug, Clone, Serialize)]
pub struct CallMeta {
    pub model_requested: String,
    pub model_used: String,
    pub prompt_hash: String,
    pub temperature: f32,
    pub seed: Option<i64>,
}

fn call_meta(
    model: &str,
    messages: &[ChatMessage],
    temperature: f32,
    seed: Option<i64>,
    result: &lmstudio::ChatResult,
) -> CallMeta {
    let system = messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.as_str())
        .unwrap_or("");
    CallMeta {
        model_requested: model.to_string(),
        model_used: result
            .model
            .as_deref()
            .filter(|m| !m.is_empty())
            .unwrap_or(model)
            .to_string(),
        prompt_hash: crate::transparency::prompt_fingerprint(system),
        temperature,
        seed,
    }
}

/// The default persona shared by every AI rater (screening + coding). It frames
/// the model as a careful SLR researcher and is editable by the user in Settings
/// (`ai_raters.system_prompt`). Task-specific instructions (JSON shape, criteria,
/// rationale rules) are appended by each function after this persona. When the
/// stored setting is blank, this constant is used as the fallback.
pub const DEFAULT_RATER_SYSTEM_PROMPT: &str = "You are a meticulous researcher conducting a systematic literature review to identify themes in the literature, following PRISMA 2020 and established systematic-review (SLR) principles. You reason strictly from the evidence placed in front of you — you never fabricate findings, invent details, or infer beyond what the text supports. You commit to a decision only when the evidence makes you genuinely confident; when the evidence is insufficient, you say so rather than guess. You are precise, consistent, and transparent about the reasoning behind every judgement.";

/// Returns the trimmed base persona, falling back to [`DEFAULT_RATER_SYSTEM_PROMPT`]
/// when the caller passes an empty/whitespace-only override.
fn rater_base_prompt(base_prompt: &str) -> &str {
    if base_prompt.trim().is_empty() {
        DEFAULT_RATER_SYSTEM_PROMPT
    } else {
        base_prompt.trim()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Suggestion {
    #[serde(alias = "codes", alias = "name", alias = "label")]
    pub code: String,
    #[serde(default, alias = "rationales", alias = "reason", alias = "reasoning", alias = "explanation")]
    pub rationale: Option<String>,
    #[serde(default, alias = "isNew", alias = "new")]
    pub is_new: bool,
    /// The model's own 0–1 estimate. Never used to filter or rank codes.
    #[serde(default, deserialize_with = "de_confidence")]
    pub confidence: Option<f64>,
}

/// Lenient confidence parser: numbers or numeric strings; 1 < v ≤ 100 is read
/// as a percentage; the result is clamped to 0–1. Anything else is `None`.
fn de_confidence<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    let v = Option::<serde_json::Value>::deserialize(d)?;
    let n = match v {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.trim().trim_end_matches('%').parse::<f64>().ok(),
        _ => None,
    };
    Ok(n.filter(|x| x.is_finite()).map(|x| {
        let x = if x > 1.0 && x <= 100.0 { x / 100.0 } else { x };
        x.clamp(0.0, 1.0)
    }))
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
    /// Earlier setup answers the prompt included.
    pub context: Vec<StepContextField>,
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
    temperature: f32,
    api_key: Option<&str>,
) -> Result<StepQuestionsResult> {
    let prompt = step_questions_prompt(step_id, current_value, project_name, slr_type, framework, domain)?;
    let (system, user, context) = (prompt.system, prompt.user, prompt.context);

    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];

    let result = lmstudio::chat(base_url, model, &messages, Some(temperature), api_key).await?;

    let questions = parse_step_questions(&result.content)
        .ok_or_else(|| anyhow!("could not parse clarifying questions from model output"))?;

    if questions.is_empty() {
        return Err(anyhow!("model returned no clarifying questions"));
    }

    Ok(StepQuestionsResult {
        questions,
        model: result.model,
        latency_ms: result.latency_ms,
        context,
    })
}

/// Generic step assistant for all 7 onboarding steps.
/// Returns the suggestion, its rationale, the model that answered and the
/// earlier setup answers sent as context. For text steps the suggestion is a refined
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
    temperature: f32,
    api_key: Option<&str>,
) -> Result<StepSuggestion> {
    let prompt = step_suggest_prompt(step_id, current_value, project_name, slr_type, framework, domain)?;
    let (system, user, context) = (prompt.system, prompt.user, prompt.context);

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

    let result = lmstudio::chat(base_url, model, &messages, Some(temperature), api_key).await?;

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
    Ok(StepSuggestion { suggestion, rationale, model: result.model, context })
}

/// An earlier setup answer included in a setup-assistant prompt, recorded in the
/// decision log so the researcher can see exactly what the model was given.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct StepContextField {
    pub label: String,
    pub value: String,
}

/// A setup-assistant prompt plus the context fields it contains.
pub struct StepPrompt {
    pub system: String,
    pub user: String,
    pub context: Vec<StepContextField>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StepSuggestion {
    pub suggestion: String,
    pub rationale: String,
    pub model: Option<String>,
    pub context: Vec<StepContextField>,
}

fn context_fields(pairs: &[(&str, &str)]) -> Vec<StepContextField> {
    pairs
        .iter()
        .map(|(label, value)| StepContextField { label: label.to_string(), value: value.to_string() })
        .collect()
}

/// Prompt for the clarifying questions the assistant asks before suggesting.
/// The returned context lists exactly the earlier answers the prompt embeds.
pub fn step_questions_prompt(
    step_id: &str,
    current_value: &str,
    project_name: &str,
    slr_type: &str,
    framework: &str,
    domain: &str,
) -> Result<StepPrompt> {
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
    let context = match step_id {
        "name" => vec![],
        "slr_type" => context_fields(&[("Title", project_name)]),
        "framework" => context_fields(&[("Title", project_name), ("Review type", slr_type), ("Domain", domain)]),
        "domain" => context_fields(&[("Title", project_name), ("Review type", slr_type), ("Framework", framework)]),
        _ => context_fields(&[("Title", project_name), ("Domain", domain), ("Framework", framework)]),
    };
    Ok(StepPrompt { system, user, context })
}

/// Prompt for the assistant's suggestion. The returned context lists exactly
/// the earlier answers the prompt embeds.
pub fn step_suggest_prompt(
    step_id: &str,
    current_value: &str,
    project_name: &str,
    slr_type: &str,
    framework: &str,
    domain: &str,
) -> Result<StepPrompt> {
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
    let context = match step_id {
        "name" => vec![],
        "slr_type" => context_fields(&[("Title", project_name)]),
        "framework" => context_fields(&[("Title", project_name), ("Review type", slr_type), ("Domain", domain)]),
        "domain" => context_fields(&[("Title", project_name), ("Review type", slr_type), ("Framework", framework)]),
        _ => context_fields(&[
            ("Title", project_name),
            ("Review type", slr_type),
            ("Framework", framework),
            ("Domain", domain),
        ]),
    };
    Ok(StepPrompt { system, user, context })
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
    temperature: f32,
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

    let result = lmstudio::chat(base_url, model, &messages, Some(temperature), api_key).await?;

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
    temperature: f32,
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

    let result = lmstudio::chat(base_url, model, &messages, Some(temperature), api_key).await?;

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

    const LEGACY_MERGING: &str = "You are an independent merging agent for a systematic literature review. \
Two coders (Rater 1 and Rater 2) each proposed codes for the same dimension of the same \
article, and these did not match. Your only job is to decide which Rater 1 / Rater 2 PAIRS \
denote the SAME underlying concept — for example two names for the same research context, or \
a general method and a specific tool that implements it (\"Structural equation modeling\" and \
\"SmartPLS analysis\").\n\n\
Rules:\n\
- Only pair codes you are confident mean the same thing. If unsure, leave them out.\n\
- For each pair, choose a canonical label that is EXACTLY one of the two given labels (the \
clearer or more standard one). Never invent a new label.\n\
- Do not pair codes that are genuinely different concepts.\n\n\
Respond ONLY with a JSON array (possibly empty). Each element: \
{\"rater1_label\": string, \"rater2_label\": string, \"canonical_label\": string, \"rationale\": string}. \
No prose before or after the JSON.";
    const LEGACY_SYNTHESIS: &str = "You are writing the Results/synthesis section of a systematic literature \
review. Write a structured narrative ORGANISED BY THE FRAMEWORK DIMENSIONS provided. For each \
dimension, synthesise what the corpus says — drawing on the coded themes and the excerpts — \
noting agreements, tensions, and gaps. \
Cite sources inline by their title, e.g. 'Smith et al. found …'. Only state what the excerpts \
support; do not invent findings. Use markdown: a '##' heading per dimension, prose and bullets \
beneath. No preamble before the first heading.";
    const LEGACY_CONSOLIDATING: &str = "You are consolidating the codebook for a systematic literature review. \
You are given the list of codes researchers and AI raters applied within ONE dimension across \
all articles. Some are near-duplicates that denote the SAME concept worded differently \
(e.g. \"Structural equation modelling\" / \"Structural equation modeling\", \"Higher education \
setting\" / \"Business education\"). Group those into clusters so the codebook is clean.\n\n\
Rules:\n\
- Only cluster codes you are confident denote the same concept. Leave genuinely distinct codes \
out (a code with no synonym simply doesn't appear in your output).\n\
- For each cluster pick a canonical label that is EXACTLY one of the given labels. Never invent a \
new label.\n\
- If a cluster contains a code marked [researcher-defined], that label MUST be the canonical one.\n\n\
Respond ONLY with a JSON array (possibly empty). Each element: \
{\"canonical_label\": string, \"variant_labels\": [string, ...]} where variant_labels are the \
OTHER labels in the cluster (not equal to canonical_label). No prose before or after the JSON.";

    #[test]
    fn defaults_reproduce_legacy_prompts() {
        assert_eq!(agent_prompt("", DEFAULT_CONSOLIDATING_INSTRUCTIONS, CONSOLIDATING_OUTPUT_FORMAT), LEGACY_CONSOLIDATING);
        assert_eq!(agent_prompt("", DEFAULT_SYNTHESIS_INSTRUCTIONS, SYNTHESIS_OUTPUT_FORMAT), LEGACY_SYNTHESIS);
        assert_eq!(agent_prompt("", DEFAULT_MERGING_INSTRUCTIONS, MERGING_OUTPUT_FORMAT), LEGACY_MERGING);
    }

    #[test]
    fn custom_instructions_keep_fixed_format() {
        let p = agent_prompt("Be strict.", DEFAULT_MERGING_INSTRUCTIONS, MERGING_OUTPUT_FORMAT);
        assert!(p.starts_with("Be strict."));
        assert!(p.ends_with(MERGING_OUTPUT_FORMAT));
    }

    #[test]
    fn blank_custom_uses_default() {
        assert_eq!(agent_prompt("  \n", "D", "F"), agent_prompt("", "D", "F"));
    }

    #[test]
    fn parse_query_result_canonical_keys() {
        let s = r#"{"query": "X AND Y", "rationale": "because"}"#;
        let (q, r) = parse_query_result(s).unwrap();
        assert_eq!(q, "X AND Y");
        assert_eq!(r, "because");
    }

    #[test]
    fn confidence_parsed_and_clamped() {
        let p = |c: &str| {
            let j = format!(r#"{{"Method":[{{"code":"Survey","rationale":"x",{c}"is_new":false}}]}}"#);
            parse_dimension_codings(&j, &["Method"]).unwrap()[0].suggestions[0].confidence
        };
        assert_eq!(p(r#""confidence":85,"#), Some(0.85));
        assert_eq!(p(r#""confidence":0.4,"#), Some(0.4));
        assert_eq!(p(r#""confidence":250,"#), Some(1.0));
        assert_eq!(p(r#""confidence":-1,"#), Some(0.0));
        assert_eq!(p(""), None);
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
/// `temperature` controls sampling — 0.0 makes the model fully deterministic so the same
/// prompt produces the same decision every call (useful when raters have identical profiles
/// and you want disagreement to come from profile differences, not sampling noise).
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
    base_prompt: &str,
    profile: &str,
    temperature: f32,
    seed: Option<i64>,
    api_key: Option<&str>,
) -> Result<(ScreeningDecision, CallMeta)> {
    let base = rater_base_prompt(base_prompt);
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
        "{base}{profile_block}\n\n\
         YOUR CURRENT TASK: title-and-abstract level screening, \
         with the article's publication metadata available alongside the abstract. \
         You decide whether each article should be included, excluded, or flagged as uncertain.\n\n\
         Domain scope of this review:\n{domain_block}\n\n\
         Exclusion criteria (exclude the article if it meets ANY of these):\n{exc_block}\n\n\
         When a criterion concerns publication metadata (journal name, publication year, authors), \
         use the metadata fields below — not just the abstract — to evaluate it. When a criterion \
         is about content or scope, read the abstract.\n\n\
         RATIONALE REQUIREMENTS (read carefully — your rationale is logged in the audit trail and \
         a human researcher will read it):\n\
         - 2–4 complete sentences. Never one bare clause.\n\
         - Cite specific evidence: quote a phrase from the title or abstract, name the journal, \
           reference the year, etc. Generic statements like 'the article is relevant' or \
           'matches the domain' are unacceptable.\n\
         - For an exclude decision, state which criterion text matched and why the evidence \
           triggered it.\n\
         - For an include decision, identify which aspects of the abstract or metadata place \
           the article inside the domain scope.\n\
         - For uncertain, name what specific information you would need to decide and why the \
           current input is insufficient.\n\
         - Do NOT pad with filler. Do NOT restate the criteria list verbatim. Do NOT mention \
           your confidence number in the rationale text.\n\n\
         Respond ONLY with a JSON object with exactly four keys:\n\
         - \"decision\": \"include\" if the article is relevant to the domain scope and passes all exclusion criteria, \
         \"exclude\" if it meets any exclusion criterion, or \"uncertain\" if relevance is unclear\n\
         - \"criterion_number\": the integer EC number if decision is \"exclude\", otherwise null\n\
         - \"rationale\": grounded explanation following the requirements above\n\
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

    let result = lmstudio::chat_seeded(base_url, model, &messages, Some(temperature), seed, api_key).await?;
    let meta = call_meta(model, &messages, temperature, seed, &result);

    let mut decision = parse_screening_decision(&result.content)
        .ok_or_else(|| anyhow::anyhow!("could not parse screening decision from model output: {}", result.content))?;

    // Validate the rationale — a confidence with no real reasoning is useless
    // for an audit trail. If the model returned a trivially short or generic
    // rationale, retry once with a sharper nudge.
    if !is_substantive_rationale(&decision.rationale) {
        let retry_messages = vec![
            ChatMessage { role: "system".into(), content: messages[0].content.clone() },
            ChatMessage { role: "user".into(), content: messages[1].content.clone() },
            ChatMessage {
                role: "assistant".into(),
                content: serde_json::to_string(&serde_json::json!({
                    "decision": decision.decision,
                    "criterion_number": decision.criterion_number,
                    "rationale": decision.rationale,
                    "confidence": decision.confidence,
                })).unwrap_or_default(),
            },
            ChatMessage {
                role: "user".into(),
                content: "Your rationale is too short or generic for an audit trail. Rewrite it as 2–4 complete sentences that cite specific evidence from the title, abstract, journal, year, or authors above. Do not change your decision or confidence. Return the same JSON shape with only the rationale rewritten.".into(),
            },
        ];
        if let Ok(retry) = lmstudio::chat_seeded(base_url, model, &retry_messages, Some(temperature), seed, api_key).await {
            if let Some(retried) = parse_screening_decision(&retry.content) {
                if is_substantive_rationale(&retried.rationale) {
                    decision.rationale = retried.rationale;
                }
            }
        }
        // If retry also failed to produce a substantive rationale, prefix a marker so
        // a downstream reviewer knows the audit trail is weak rather than missing.
        if !is_substantive_rationale(&decision.rationale) {
            decision.rationale = format!(
                "[weak rationale — recommend manual review] {}",
                decision.rationale.trim()
            );
        }
    }

    Ok((decision, meta))
}

/// Full-text (eligibility) screening: reads the article's full text and decides
/// include / exclude / uncertain against the criteria. Mirrors `screen_article`
/// but at the full-text level.
#[allow(clippy::too_many_arguments)]
pub async fn screen_article_fulltext(
    base_url: &str,
    model: &str,
    title: &str,
    full_text: &str,
    domain: &str,
    exclusion_criteria_labeled: &[(i64, String)],
    base_prompt: &str,
    profile: &str,
    temperature: f32,
    seed: Option<i64>,
    api_key: Option<&str>,
) -> Result<(ScreeningDecision, CallMeta)> {
    let base = rater_base_prompt(base_prompt);
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
    // Truncate to keep the prompt manageable (char-safe).
    let body: String = full_text.chars().take(12_000).collect();

    let system = format!(
        "{base}{profile_block}\n\n\
YOUR CURRENT TASK: FULL-TEXT eligibility screening. \
You read the full text of an article that already passed title-and-abstract screening, and decide \
whether it is eligible for inclusion, should be excluded, or is uncertain.\n\n\
Domain scope of this review:\n{domain_block}\n\n\
Exclusion criteria (exclude the article if it meets ANY of these):\n{exc_block}\n\n\
Base your decision on the FULL TEXT below — its methods, sample, setting, and findings — not just \
the abstract. Provide a 2–4 sentence rationale citing specific evidence from the text (a method, a \
sample description, a finding). For an exclude, name which criterion matched and the evidence.\n\n\
Respond ONLY with a JSON object: {{\"decision\": \"include\"|\"exclude\"|\"uncertain\", \
\"criterion_number\": number-or-null, \"rationale\": string, \"confidence\": number-0-to-1}}. \
No prose before or after the JSON."
    );
    let user = format!("Title: {title}\n\nFull text (excerpt):\n{body}\n\nReturn the JSON decision.");
    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];
    let result = lmstudio::chat_seeded(base_url, model, &messages, Some(temperature), seed, api_key).await?;
    let meta = call_meta(model, &messages, temperature, seed, &result);
    let decision = parse_screening_decision(&result.content).ok_or_else(|| {
        anyhow::anyhow!("could not parse full-text screening decision from: {}", result.content)
    })?;
    Ok((decision, meta))
}

/// A rationale is substantive when it has at least ~40 non-whitespace characters
/// AND is not one of a small set of generic filler phrases the model produces
/// when it skips the reasoning step. The threshold is conservative: a single
/// sentence with specific evidence is usually 40+ characters; trivial responses
/// like "Relevant to the domain." or "Matches the criteria." don't meet it.
fn is_substantive_rationale(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.chars().filter(|c| !c.is_whitespace()).count() < 40 {
        return false;
    }
    let lower = trimmed.to_lowercase();
    let trivial_starts = [
        "relevant to the domain",
        "matches the criteria",
        "fits the scope",
        "no rationale",
        "see above",
        "as stated",
        "n/a",
    ];
    if trivial_starts.iter().any(|p| lower.starts_with(p)) && trimmed.len() < 80 {
        return false;
    }
    true
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
    base_prompt: &str,
    profile: &str,
    // When true, treat each dimension's existing codes as a FIXED codebook and
    // select from it (closed-set coding); new codes only as a rare exception.
    // When false, open generation (used to build the codebook in the first place).
    closed_set: bool,
    temperature: f32,
    seed: Option<i64>,
    api_key: Option<&str>,
) -> Result<(Vec<DimensionCoding>, CallMeta)> {
    let base = rater_base_prompt(base_prompt);
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

    let task_instruction = if closed_set {
        "For each dimension, treat its \"Existing codes\" list as a FIXED CODEBOOK. \
SELECT every code from that list that applies to this article and copy its label VERBATIM \
(same wording, no rephrasing). Apply zero, one, or several per dimension. \
Only propose a new code (is_new=true) when the article clearly expresses something the codebook \
cannot capture — strongly prefer reusing a codebook label over inventing one."
    } else {
        "For each dimension you will suggest 1–3 short noun-phrase codes (2–5 words each). \
Reuse an existing code when it fits; only mark is_new=true when no existing code is appropriate."
    };
    let system = format!(
        "{base}{coding_profile_block}\n\n\
YOUR CURRENT TASK: code articles for a systematic literature review against an organising \
framework. You suggest codes (themes) — you do not make final decisions. The researcher always has final say.\n\n\
Review domain: {domain}\nOrganising framework: {framework}\nResearch questions:\n{rqs_block}\n\n\
{task_instruction}\n\n\
Respond ONLY with a JSON object. The keys are the exact dimension names provided. \
Each value is a JSON array of objects with keys: code (string), rationale (string, one sentence), confidence (number 0.0–1.0, your own estimate of how well the text supports this code), is_new (boolean).\n\
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

    let result = lmstudio::chat_seeded(base_url, model, &messages, Some(temperature), seed, api_key).await?;
    let meta = call_meta(model, &messages, temperature, seed, &result);

    let codings = parse_dimension_codings(&result.content, &dim_names)
        .ok_or_else(|| anyhow::anyhow!("could not parse dimension codings from: {}", &result.content))?;

    Ok((codings, meta))
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

// ── Disagreement adjudication ───────────────────────────────────────────────
// An independent model decides which of two raters' leftover codes (within one
// dimension of one article) denote the SAME concept, so the easy disagreements
// resolve automatically. It may only pick a canonical label that is one of the
// two inputs — it never invents a third.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdjudicationResolution {
    #[serde(alias = "rater1", alias = "label1", alias = "r1")]
    pub rater1_label: String,
    #[serde(alias = "rater2", alias = "label2", alias = "r2")]
    pub rater2_label: String,
    #[serde(alias = "canonical", alias = "label", alias = "chosen")]
    pub canonical_label: String,
    #[serde(default, alias = "reason", alias = "reasoning", alias = "explanation")]
    pub rationale: String,
}

// ── Editable agent instructions + fixed output formats ──────────────────────
// The researcher may edit the instructions of the merging, consolidating and
// synthesis agents. The output-format tail is fixed so parsing keeps working.

pub const DEFAULT_MERGING_INSTRUCTIONS: &str = "You are an independent merging agent for a systematic literature review. \
Two coders (Rater 1 and Rater 2) each proposed codes for the same dimension of the same \
article, and these did not match. Your only job is to decide which Rater 1 / Rater 2 PAIRS \
denote the SAME underlying concept — for example two names for the same research context, or \
a general method and a specific tool that implements it (\"Structural equation modeling\" and \
\"SmartPLS analysis\").\n\n\
Rules:\n\
- Only pair codes you are confident mean the same thing. If unsure, leave them out.\n\
- For each pair, choose a canonical label that is EXACTLY one of the two given labels (the \
clearer or more standard one). Never invent a new label.\n\
- Do not pair codes that are genuinely different concepts.";
pub const MERGING_OUTPUT_FORMAT: &str = "Respond ONLY with a JSON array (possibly empty). Each element: \
{\"rater1_label\": string, \"rater2_label\": string, \"canonical_label\": string, \"rationale\": string}. \
No prose before or after the JSON.";

pub const DEFAULT_CONSOLIDATING_INSTRUCTIONS: &str = "You are consolidating the codebook for a systematic literature review. \
You are given the list of codes researchers and AI raters applied within ONE dimension across \
all articles. Some are near-duplicates that denote the SAME concept worded differently \
(e.g. \"Structural equation modelling\" / \"Structural equation modeling\", \"Higher education \
setting\" / \"Business education\"). Group those into clusters so the codebook is clean.\n\n\
Rules:\n\
- Only cluster codes you are confident denote the same concept. Leave genuinely distinct codes \
out (a code with no synonym simply doesn't appear in your output).\n\
- For each cluster pick a canonical label that is EXACTLY one of the given labels. Never invent a \
new label.\n\
- If a cluster contains a code marked [researcher-defined], that label MUST be the canonical one.";
pub const CONSOLIDATING_OUTPUT_FORMAT: &str = "Respond ONLY with a JSON array (possibly empty). Each element: \
{\"canonical_label\": string, \"variant_labels\": [string, ...]} where variant_labels are the \
OTHER labels in the cluster (not equal to canonical_label). No prose before or after the JSON.";

pub const DEFAULT_SYNTHESIS_INSTRUCTIONS: &str = "You are writing the Results/synthesis section of a systematic literature \
review. Write a structured narrative ORGANISED BY THE FRAMEWORK DIMENSIONS provided. For each \
dimension, synthesise what the corpus says — drawing on the coded themes and the excerpts — \
noting agreements, tensions, and gaps. \
Cite sources inline by their title, e.g. 'Smith et al. found …'. Only state what the excerpts \
support; do not invent findings.";
pub const SYNTHESIS_OUTPUT_FORMAT: &str = "Use markdown: a '##' heading per dimension, prose and bullets \
beneath. No preamble before the first heading.";

/// Build an agent's system prompt: custom instructions (blank -> default) followed
/// by the fixed output format. Keeps today's exact whitespace between the two.
pub fn agent_prompt(custom: &str, default: &str, format: &str) -> String {
    let instructions = if custom.trim().is_empty() { default } else { custom.trim() };
    let sep = if format == SYNTHESIS_OUTPUT_FORMAT { " " } else { "\n\n" };
    format!("{instructions}{sep}{format}")
}

/// Ask the merging agent which `(rater1, rater2)` label pairs mean the same thing.
/// `rater1`/`rater2` are `(label, rationale)` lists of each rater's leftover
/// codes for one dimension of one article. Returns only the pairs the model is
/// confident are equivalent.
#[allow(clippy::too_many_arguments)]
pub async fn adjudicate_disagreements(
    base_url: &str,
    model: &str,
    dimension: &str,
    rater1: &[(String, String)],
    rater2: &[(String, String)],
    instructions: &str,
    temperature: f32,
    seed: Option<i64>,
    api_key: Option<&str>,
) -> Result<(Vec<AdjudicationResolution>, CallMeta)> {
    let fmt = |items: &[(String, String)]| {
        items
            .iter()
            .map(|(label, rat)| {
                if rat.trim().is_empty() {
                    format!("  - {label}")
                } else {
                    format!("  - {label} — {rat}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let system = agent_prompt(instructions, DEFAULT_MERGING_INSTRUCTIONS, MERGING_OUTPUT_FORMAT);

    let user = format!(
        "Dimension: {dimension}\n\nRater 1 codes:\n{}\n\nRater 2 codes:\n{}\n\nReturn the JSON array now.",
        fmt(rater1),
        fmt(rater2),
    );

    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];

    let result = lmstudio::chat_seeded(base_url, model, &messages, Some(temperature), seed, api_key).await?;
    let meta = call_meta(model, &messages, temperature, seed, &result);
    let resolutions = parse_adjudications(&result.content)
        .ok_or_else(|| anyhow!("could not parse adjudications from: {}", &result.content))?;
    Ok((resolutions, meta))
}

// ── Research agenda proposal (SPAR-4-SLR Evaluation 3a) ─────────────────────

/// Draft a future-research agenda from the review's questions, framework, and the
/// dimension→code frequency table. Returns markdown (bulleted directions).
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub async fn generate_research_agenda(
    base_url: &str,
    model: &str,
    domain: &str,
    framework: &str,
    research_questions: &[&str],
    dims_with_codes: &[(String, Vec<(String, i64)>)],
    guidance: Option<&str>,
    revise_current: Option<&str>,
    revise_instruction: Option<&str>,
    temperature: f32,
    api_key: Option<&str>,
) -> Result<String> {
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

    let findings_block = dims_with_codes
        .iter()
        .map(|(dim, codes)| {
            let codes_str = if codes.is_empty() {
                "(no codes applied)".to_string()
            } else {
                codes
                    .iter()
                    .map(|(c, n)| format!("{c} ({n})"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!("- {dim}: {codes_str}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let context = format!(
        "Domain: {domain}\nFramework: {framework}\nResearch questions:\n{rqs_block}\n\n\
Codes applied per dimension (with counts):\n{findings_block}"
    );

    let (system, user) = if let (Some(cur), Some(instr)) = (revise_current, revise_instruction) {
        // Revision mode — rework the existing agenda per the instruction.
        let system = "You are revising a future-research agenda for a systematic literature \
review. Apply the researcher's revision instruction faithfully while keeping every direction \
specific and grounded in the review's questions and findings. Return the FULL revised agenda \
as markdown bullet points only, no preamble."
            .to_string();
        let user = format!(
            "{context}\n\nCurrent agenda:\n{cur}\n\nRevision instruction: {instr}\n\nReturn the full revised agenda now."
        );
        (system, user)
    } else {
        let system = "You are a research methodologist helping a scholar write the future-research \
agenda for a systematic literature review (SPAR-4-SLR, Assessing stage). You are given the \
review's domain, organising framework, research questions, and the frequency of codes applied \
across each framework dimension. Codes that appear rarely or unevenly across dimensions often \
signal gaps and opportunities.\n\n\
Write a concise future-research agenda as 4–7 markdown bullet points. Each bullet states a \
specific, defensible research direction grounded in the dimensions and gaps shown — not generic \
advice. Where useful, frame a direction as a question. Respond with the markdown bullets only, \
no preamble."
            .to_string();
        let mut user = context.clone();
        if let Some(g) = guidance.filter(|g| !g.trim().is_empty()) {
            user.push_str(&format!(
                "\n\nThe researcher's priorities for the agenda (weave these in):\n{}",
                g.trim()
            ));
        }
        user.push_str("\n\nWrite the agenda now.");
        (system, user)
    };

    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];

    let result = lmstudio::chat(base_url, model, &messages, Some(temperature), api_key).await?;
    Ok(result.content.trim().to_string())
}

/// Write a structured, cited synthesis narrative organised by framework
/// dimension. `dims` is `(dimension, codes, excerpts)` where excerpts are
/// `(source_title, text)` retrieved from the corpus for that dimension.
#[allow(clippy::too_many_arguments)]
pub async fn generate_synthesis(
    base_url: &str,
    model: &str,
    domain: &str,
    framework: &str,
    research_questions: &[&str],
    dims: &[(String, Vec<String>, Vec<(String, String)>)],
    instructions: &str,
    temperature: f32,
    seed: Option<i64>,
    api_key: Option<&str>,
) -> Result<(String, CallMeta)> {
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

    let mut blocks = String::new();
    for (dim, codes, excerpts) in dims {
        blocks.push_str(&format!("\n## Dimension: {dim}\n"));
        if !codes.is_empty() {
            blocks.push_str(&format!("Coded themes in this dimension: {}\n", codes.join(", ")));
        }
        if excerpts.is_empty() {
            blocks.push_str("(no relevant excerpts retrieved)\n");
        } else {
            for (title, text) in excerpts {
                blocks.push_str(&format!("- Source \"{title}\": {text}\n"));
            }
        }
    }

    let system = agent_prompt(instructions, DEFAULT_SYNTHESIS_INSTRUCTIONS, SYNTHESIS_OUTPUT_FORMAT);

    let user = format!(
        "Domain: {domain}\nFramework: {framework}\nResearch questions:\n{rqs_block}\n\n\
Per-dimension coded themes and retrieved excerpts:\n{blocks}\n\nWrite the synthesis now."
    );

    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];

    let result = lmstudio::chat_seeded(base_url, model, &messages, Some(temperature), seed, api_key).await?;
    let meta = call_meta(model, &messages, temperature, seed, &result);
    Ok((result.content.trim().to_string(), meta))
}

// ── Corpus-level code consolidation ─────────────────────────────────────────
// Cluster synonymous codes within one dimension across the whole corpus and pick
// a canonical label per cluster (semantic judgment, not string similarity).

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConsolidationCluster {
    #[serde(alias = "canonical", alias = "label", alias = "keep")]
    pub canonical_label: String,
    #[serde(default, alias = "variants", alias = "merge", alias = "others")]
    pub variant_labels: Vec<String>,
}

/// `codes` is `(label, count, is_researcher)`. Returns clusters of codes that mean
/// the same thing; the canonical label is always one of the inputs, and a
/// researcher-typed label is preferred as canonical when present in a cluster.
pub async fn consolidate_codes(
    base_url: &str,
    model: &str,
    dimension: &str,
    codes: &[(String, i64, bool)],
    instructions: &str,
    temperature: f32,
    seed: Option<i64>,
    api_key: Option<&str>,
) -> Result<(Vec<ConsolidationCluster>, CallMeta)> {
    let list = codes
        .iter()
        .map(|(label, count, is_researcher)| {
            let tag = if *is_researcher { " [researcher-defined]" } else { "" };
            format!("  - {label} (used {count}×){tag}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let system = agent_prompt(instructions, DEFAULT_CONSOLIDATING_INSTRUCTIONS, CONSOLIDATING_OUTPUT_FORMAT);

    let user = format!("Dimension: {dimension}\n\nCodes:\n{list}\n\nReturn the JSON array now.");

    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];

    let result = lmstudio::chat_seeded(base_url, model, &messages, Some(temperature), seed, api_key).await?;
    let meta = call_meta(model, &messages, temperature, seed, &result);
    let clusters = parse_consolidations(&result.content)
        .ok_or_else(|| anyhow!("could not parse consolidations from: {}", &result.content))?;
    Ok((clusters, meta))
}

fn parse_consolidations(s: &str) -> Option<Vec<ConsolidationCluster>> {
    let bytes = s.as_bytes();
    let start = bytes.iter().position(|&b| b == b'[')?;
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
        if c == b'[' { depth += 1; }
        else if c == b']' {
            depth -= 1;
            if depth == 0 { end = Some(i + 1); break; }
        }
    }
    let end = end?;
    serde_json::from_str::<Vec<ConsolidationCluster>>(&s[start..end]).ok()
}

/// Extract the first balanced JSON array from the model output and parse it.
fn parse_adjudications(s: &str) -> Option<Vec<AdjudicationResolution>> {
    let bytes = s.as_bytes();
    let start = bytes.iter().position(|&b| b == b'[')?;
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
        if c == b'[' { depth += 1; }
        else if c == b']' {
            depth -= 1;
            if depth == 0 { end = Some(i + 1); break; }
        }
    }
    let end = end?;
    serde_json::from_str::<Vec<AdjudicationResolution>>(&s[start..end]).ok()
}

#[cfg(test)]
mod step_prompt_tests {
    use super::{step_questions_prompt, step_suggest_prompt};

    const STEPS: [&str; 7] = ["name", "slr_type", "framework", "domain", "rq_what", "rq_how", "rq_where"];
    const TITLE: &str = "ZZTITLE";
    const TYPE: &str = "ZZTYPE";
    const FRAMEWORK: &str = "ZZFRAMEWORK";
    const DOMAIN: &str = "ZZDOMAIN";

    /// The logged context must list exactly the earlier answers the prompt sends:
    /// every listed value appears in the prompt, and no unlisted one does.
    fn assert_context_matches(user: &str, context: &[super::StepContextField], step: &str) {
        let listed: Vec<&str> = context.iter().map(|c| c.value.as_str()).collect();
        for v in [TITLE, TYPE, FRAMEWORK, DOMAIN] {
            assert_eq!(
                user.contains(v),
                listed.contains(&v),
                "step {step}: value {v} in prompt = {}, in logged context = {}",
                user.contains(v),
                listed.contains(&v)
            );
        }
    }

    #[test]
    fn questions_context_matches_prompt_for_every_step() {
        for step in STEPS {
            let p = step_questions_prompt(step, "draft", TITLE, TYPE, FRAMEWORK, DOMAIN).unwrap();
            assert_context_matches(&p.user, &p.context, step);
        }
    }

    #[test]
    fn suggestion_context_matches_prompt_for_every_step() {
        for step in STEPS {
            let p = step_suggest_prompt(step, "draft", TITLE, TYPE, FRAMEWORK, DOMAIN).unwrap();
            assert_context_matches(&p.user, &p.context, step);
        }
    }

    #[test]
    fn unknown_step_is_rejected() {
        assert!(step_questions_prompt("nope", "", "", "", "", "").is_err());
        assert!(step_suggest_prompt("nope", "", "", "", "", "").is_err());
    }
}
