//! Decision-log entries for the AI agents in the review workflow (query agent,
//! codebook builder, synthesis, research agenda).
//!
//! Each model output is one append-only entry marked `ai`, carrying the model
//! used, prompt fingerprint, temperature and seed (also stored as an `ai_call`
//! row). Everything the researcher then does with it — accept, undo, dismiss,
//! save, open — is a separate entry marked `human` that points back to the AI
//! entry by its decision-log id.

use crate::db::ProjectDb;
use crate::suggest::CallMeta;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

/// Agents whose outputs are logged.
pub const AGENTS: [&str; 4] = ["query", "codebook", "synthesis", "agenda"];

/// What the researcher can do with an agent's output.
pub const ACTIONS: [&str; 6] = ["accepted", "undone", "dismissed", "saved", "opened", "edited"];

pub fn output_key(agent: &str) -> String {
    format!("agent.{agent}.output")
}

pub fn action_key(agent: &str) -> String {
    format!("agent.{agent}.action")
}

/// The AI entry's value: the agent's own fields plus provenance.
pub fn output_value(agent: &str, ai_call_id: i64, meta: &CallMeta, mut output: Value) -> Value {
    let prov = json!({
        "agent": agent,
        "ai_call_id": ai_call_id,
        "model_requested": meta.model_requested,
        "model_used": meta.model_used,
        "prompt_hash": meta.prompt_hash,
        "temperature": meta.temperature,
        "seed": meta.seed,
    });
    if let (Some(o), Some(p)) = (output.as_object_mut(), prov.as_object()) {
        for (k, v) in p {
            o.insert(k.clone(), v.clone());
        }
        output
    } else {
        json!({ "output": output, "provenance": prov })
    }
}

/// The human entry's value, after checking the agent and action are known.
pub fn action_value(agent: &str, output_id: i64, action: &str, details: Value) -> Result<Value> {
    if !AGENTS.contains(&agent) {
        return Err(anyhow!("unknown agent: {agent}"));
    }
    if !ACTIONS.contains(&action) {
        return Err(anyhow!("unknown action: {action}"));
    }
    Ok(json!({ "agent": agent, "output_id": output_id, "action": action, "details": details }))
}

/// Log an agent's output (AI). Returns the decision-log id that later
/// researcher actions refer to.
pub async fn log_output(
    db: &ProjectDb,
    stage: &str,
    agent: &str,
    purpose: &str,
    meta: &CallMeta,
    output: Value,
    rationale: Option<&str>,
) -> Result<i64> {
    let ai_call_id = db.insert_ai_call(purpose, meta).await?;
    let value = output_value(agent, ai_call_id, meta, output);
    db.append_decision_log_entry(stage, Some(agent), &output_key(agent), &value, rationale, "ai")
        .await
}

/// Log what the researcher did with an agent's output (human).
pub async fn log_action(
    db: &ProjectDb,
    stage: &str,
    agent: &str,
    output_id: i64,
    action: &str,
    details: Value,
) -> Result<i64> {
    let value = action_value(agent, output_id, action, details)?;
    db.append_decision_log_entry(stage, Some(agent), &action_key(agent), &value, None, "human")
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;

    fn meta() -> CallMeta {
        CallMeta {
            model_requested: "qwen3".into(),
            model_used: "qwen3-8b-q4".into(),
            prompt_hash: "abc123abc123".into(),
            temperature: 0.3,
            seed: Some(7),
        }
    }

    #[test]
    fn output_value_carries_provenance() {
        let v = output_value("query", 5, &meta(), json!({ "suggestion": "A AND B" }));
        assert_eq!(v["suggestion"], "A AND B");
        assert_eq!(v["ai_call_id"], 5);
        assert_eq!(v["model_used"], "qwen3-8b-q4");
        assert_eq!(v["prompt_hash"], "abc123abc123");
        assert_eq!(v["seed"], 7);
    }

    #[test]
    fn unknown_agent_or_action_is_rejected() {
        assert!(action_value("query", 1, "accepted", json!({})).is_ok());
        assert!(action_value("chat", 1, "accepted", json!({})).is_err());
        assert!(action_value("query", 1, "approved", json!({})).is_err());
    }

    #[tokio::test]
    async fn output_is_ai_and_action_is_human_and_linked() {
        let (db, _d) = test_db().await;
        let out = log_output(&db, "assembling", "query", "query", &meta(), json!({ "suggestion": "A" }), Some("why"))
            .await
            .unwrap();
        log_action(&db, "assembling", "query", out, "accepted", json!({ "query": "A" })).await.unwrap();
        let log = db.list_decision_log().await.unwrap();
        let o = log.iter().find(|d| d.decision_key == "agent.query.output").unwrap();
        let a = log.iter().find(|d| d.decision_key == "agent.query.action").unwrap();
        assert_eq!((o.actor.as_str(), a.actor.as_str()), ("ai", "human"));
        assert_eq!(o.rationale.as_deref(), Some("why"));
        let av: Value = serde_json::from_str(&a.value_json).unwrap();
        assert_eq!(av["output_id"], out);
        assert!(log.iter().all(|d| d.superseded_by.is_none()));
    }
}
