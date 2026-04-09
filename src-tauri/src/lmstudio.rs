//! Localhost-only client for LM Studio (and any OpenAI-compatible local server).
//!
//! Network policy: this client refuses to connect to anything that isn't a
//! loopback host. Connector-based external API access lives elsewhere and
//! is gated behind an explicit allowlist.

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

fn ensure_loopback(base_url: &str) -> Result<Url> {
    let url = Url::parse(base_url).map_err(|e| anyhow!("invalid base url: {e}"))?;
    let host = url.host_str().unwrap_or("");
    let is_loopback = matches!(host, "127.0.0.1" | "localhost" | "::1");
    if !is_loopback {
        return Err(anyhow!(
            "refusing non-loopback host '{host}': local inference only"
        ));
    }
    Ok(url)
}

fn http_client() -> Result<Client> {
    Ok(Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(180))
        .build()?)
}

pub async fn list_models(base_url: &str) -> Result<Vec<LmModel>> {
    let mut url = ensure_loopback(base_url)?;
    // OpenAI-compat: /v1/models
    if !url.path().ends_with("/models") {
        let path = url.path().trim_end_matches('/').to_string() + "/models";
        url.set_path(&path);
    }
    let client = http_client()?;
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("models endpoint returned {}", resp.status()));
    }
    let parsed: LmModelsResponse = resp.json().await?;
    Ok(parsed.data)
}

/// Strip `<think>...</think>` blocks (and a few common variants) from a
/// reasoning model's response so the displayed answer is just the conclusion.
/// Returns (cleaned, reasoning_trace).
pub fn split_reasoning(raw: &str) -> (String, Option<String>) {
    // Handle <think>...</think> first
    if let Some(start) = raw.find("<think>") {
        if let Some(end) = raw.find("</think>") {
            if end > start {
                let trace = raw[start + 7..end].to_string();
                let mut cleaned = String::new();
                cleaned.push_str(&raw[..start]);
                cleaned.push_str(&raw[end + 8..]);
                return (cleaned.trim().to_string(), Some(trace.trim().to_string()));
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
) -> Result<ChatResult> {
    let mut url = ensure_loopback(base_url)?;
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
    let resp = client.post(url).json(&body).send().await?;
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

pub async fn embed(base_url: &str, model: &str, inputs: &[&str]) -> Result<Vec<Vec<f32>>> {
    let mut url = ensure_loopback(base_url)?;
    if !url.path().ends_with("/embeddings") {
        let path = url.path().trim_end_matches('/').to_string() + "/embeddings";
        url.set_path(&path);
    }
    let client = http_client()?;
    let body = EmbedRequest {
        model,
        input: inputs.to_vec(),
    };
    let resp = client.post(url).json(&body).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("embeddings returned {}", resp.status()));
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
