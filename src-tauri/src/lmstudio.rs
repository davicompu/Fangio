//! Client for LM Studio and any OpenAI-compatible inference server (local or remote).

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LmModel {
    pub id: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub owned_by: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LmModelsResponse {
    pub data: Vec<LmModel>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: ChatMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatUsage {
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub completion_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
    #[serde(default)]
    pub usage: Option<ChatUsage>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResult {
    /// Final message content with reasoning traces stripped
    pub content: String,
    /// Raw reasoning trace (if any)
    pub reasoning: Option<String>,
    pub model: Option<String>,
    pub latency_ms: u64,
    pub usage: Option<ChatUsage>,
}

#[derive(Debug, Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: Vec<&'a str>,
}

#[derive(Debug, Deserialize)]
pub struct EmbedItem {
    pub embedding: Vec<f32>,
    #[serde(default)]
    pub index: u32,
}

#[derive(Debug, Deserialize)]
pub struct EmbedResponse {
    pub data: Vec<EmbedItem>,
    #[serde(default)]
    pub model: Option<String>,
}

fn parse_url(base_url: &str) -> Result<Url> {
    let url = Url::parse(base_url).map_err(|e| anyhow!("invalid base url: {e}"))?;
    // Only real network schemes — never file:, data:, etc. The model endpoint
    // receives paper content, so we refuse to point it at the local filesystem.
    match url.scheme() {
        "http" | "https" => Ok(url),
        other => Err(anyhow!("model endpoint must use http or https, not '{other}:'")),
    }
}

fn http_client() -> Result<Client> {
    Ok(Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(300))
        .build()?)
}

fn apply_auth(
    req: reqwest::RequestBuilder,
    api_key: Option<&str>,
) -> reqwest::RequestBuilder {
    match api_key.filter(|k| !k.is_empty()) {
        Some(key) => req.header("Authorization", format!("Bearer {key}")),
        None => req,
    }
}

pub async fn list_models(base_url: &str, api_key: Option<&str>) -> Result<Vec<LmModel>> {
    let mut url = parse_url(base_url)?;
    // OpenAI-compat: /v1/models
    if !url.path().ends_with("/models") {
        let path = url.path().trim_end_matches('/').to_string() + "/models";
        url.set_path(&path);
    }
    let client = http_client()?;
    let resp = apply_auth(client.get(url), api_key).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("models endpoint returned {}", resp.status()));
    }
    let parsed: LmModelsResponse = resp.json().await?;
    Ok(parsed.data)
}

/// Strip reasoning traces from a model response so the displayed answer is
/// just the conclusion. Handles `<think>`, `<thinking>`, and `<reasoning>`
/// tag variants used by common local models (DeepSeek-R1, Qwen, etc.).
/// Returns (cleaned, reasoning_trace).
pub fn split_reasoning(raw: &str) -> (String, Option<String>) {
    for (open, close) in &[
        ("<think>", "</think>"),
        ("<thinking>", "</thinking>"),
        ("<reasoning>", "</reasoning>"),
    ] {
        if let Some(start) = raw.find(open) {
            if let Some(end) = raw.find(close) {
                if end > start {
                    let trace = raw[start + open.len()..end].to_string();
                    let mut cleaned = String::new();
                    cleaned.push_str(&raw[..start]);
                    cleaned.push_str(&raw[end + close.len()..]);
                    return (cleaned.trim().to_string(), Some(trace.trim().to_string()));
                }
            }
        }
    }
    (raw.to_string(), None)
}

pub async fn chat(
    base_url: &str,
    model: &str,
    messages: &[ChatMessage],
    temperature: Option<f32>,
    api_key: Option<&str>,
) -> Result<ChatResult> {
    let mut url = parse_url(base_url)?;
    if !url.path().ends_with("/chat/completions") {
        let path = url.path().trim_end_matches('/').to_string() + "/chat/completions";
        url.set_path(&path);
    }
    let client = http_client()?;
    let body = ChatRequest {
        model,
        messages,
        temperature,
        max_tokens: None,
        stream: false,
    };
    let started = Instant::now();
    let resp = apply_auth(client.post(url).json(&body), api_key).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("chat returned {status}: {text}"));
    }
    let parsed: ChatResponse = resp.json().await?;
    let latency_ms = started.elapsed().as_millis() as u64;
    let raw = parsed
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();
    let (content, reasoning) = split_reasoning(&raw);
    Ok(ChatResult {
        content,
        reasoning,
        model: parsed.model,
        latency_ms,
        usage: parsed.usage,
    })
}

pub async fn embed(base_url: &str, model: &str, inputs: &[&str], api_key: Option<&str>) -> Result<Vec<Vec<f32>>> {
    let mut url = parse_url(base_url)?;
    if !url.path().ends_with("/embeddings") {
        let path = url.path().trim_end_matches('/').to_string() + "/embeddings";
        url.set_path(&path);
    }
    let client = http_client()?;
    let body = EmbedRequest {
        model,
        input: inputs.to_vec(),
    };
    let resp = apply_auth(client.post(url).json(&body), api_key).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        let trimmed = detail.trim();
        // 400 from LM Studio is almost always "model not loaded" — give the
        // user a concrete next step rather than a bare status code.
        if status.as_u16() == 400 {
            return Err(anyhow!(
                "embeddings returned 400: model '{model}' may not be loaded in LM Studio. \
                 Open LM Studio → load this embedding model, then try again. \
                 Server said: {}",
                if trimmed.is_empty() { "(no body)" } else { trimmed }
            ));
        }
        return Err(anyhow!("embeddings returned {status}: {}", if trimmed.is_empty() { "(no body)" } else { trimmed }));
    }
    let parsed: EmbedResponse = resp.json().await?;
    let mut out = vec![Vec::new(); inputs.len()];
    for item in parsed.data {
        let i = item.index as usize;
        if i < out.len() {
            out[i] = item.embedding;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod url_guard_tests {
    use super::parse_url;

    #[test]
    fn accepts_http_and_https() {
        assert!(parse_url("http://127.0.0.1:1234/v1").is_ok());
        assert!(parse_url("https://api.example.com/v1").is_ok());
    }

    #[test]
    fn rejects_non_network_schemes() {
        assert!(parse_url("file:///etc/passwd").is_err());
        assert!(parse_url("data:text/plain,hi").is_err());
        assert!(parse_url("ftp://host/x").is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_url("not a url").is_err());
    }
}
