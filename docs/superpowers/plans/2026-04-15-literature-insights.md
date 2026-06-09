# Literature Insights (Clusters + Glossary) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the "Ask the Literature" view into a three-tab interface — Chat (existing), Clusters (K-means on embeddings rendered as a mind map), and Glossary (LLM-extracted domain terms stored per-project and browsable).

**Architecture:** The existing `RagChatView.tsx` becomes a tab shell. Two new tabs are added: Clusters calls a new `cluster_corpus` Tauri command that runs K-means on the stored article embeddings (one centroid per article, averaged from its chunks) and asks the LLM to label each cluster; the result is rendered with `react-flow` as a mind-map tree. Glossary adds a `glossary_term` SQLite table populated by a new `extract_glossary` command (LLM pass over each article's chunks) and displayed as a searchable, linked list.

**Tech Stack:** Rust + SQLx + Tauri events, React + TypeScript, `@xyflow/react` (react-flow v12) for mind map, existing `lmstudio.rs` / `vector.rs` / `suggest.rs` patterns.

---

## Files Changed

| File | Change |
|---|---|
| `src-tauri/migrations/0016_glossary.sql` | Create `glossary_term` table |
| `src-tauri/src/db.rs` | Add `GlossaryTerm` struct + 6 new methods |
| `src-tauri/src/lib.rs` | Add `cluster_corpus` and `extract_glossary` commands + register |
| `src/lib/api.ts` | Add `ClusterNode`, `GlossaryTerm`, `GlossaryExtractProgress` interfaces + bridge functions |
| `src/views/RagChatView.tsx` | Refactor into three-tab shell (Chat / Clusters / Glossary) |
| `src/views/rag/ChatTab.tsx` | Extract existing chat UI into its own file |
| `src/views/rag/ClustersTab.tsx` | New: K-means result as react-flow mind map |
| `src/views/rag/GlossaryTab.tsx` | New: searchable glossary list + extract button |

---

## Task 1: Database Migration — glossary_term table

**Files:**
- Create: `src-tauri/migrations/0016_glossary.sql`

- [ ] **Step 1: Create the migration**

```sql
-- 0016_glossary.sql
-- Stores LLM-extracted domain terms for the project glossary.
-- Terms are extracted per-article and deduplicated by term text (case-insensitive).
CREATE TABLE IF NOT EXISTS glossary_term (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    term        TEXT NOT NULL,
    definition  TEXT NOT NULL,
    article_ids TEXT NOT NULL DEFAULT '[]',   -- JSON array of article IDs that use this term
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_glossary_term_lower ON glossary_term(lower(term));
```

- [ ] **Step 2: Verify the file is valid SQL**

```bash
cd /Users/david/Documents/GitHub/Fangio
sqlite3 /tmp/test_migration.db < src-tauri/migrations/0016_glossary.sql && echo "OK"
```

Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/migrations/0016_glossary.sql
git commit -m "feat(db): add glossary_term table"
```

---

## Task 2: db.rs — GlossaryTerm struct and methods

**Files:**
- Modify: `src-tauri/src/db.rs`

Context: All existing structs are in the top section of db.rs. Methods are grouped by domain. The `AiCodingProposal` struct was inserted before `ExclusionCriterionRow`. Add the new struct before `ExclusionCriterionRow` as well. Methods go in a new section at the end of the `impl ProjectDb` block (before the closing `}`).

- [ ] **Step 1: Add `GlossaryTerm` struct**

Find in db.rs:
```rust
pub struct ExclusionCriterionRow {
```

Insert before it:
```rust
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct GlossaryTerm {
    pub id: i64,
    pub term: String,
    pub definition: String,
    /// JSON array string of article IDs, e.g. "[1,3,7]"
    pub article_ids: String,
    pub created_at: String,
    pub updated_at: String,
}

```

- [ ] **Step 2: Add 6 new methods to `impl ProjectDb`**

Find the last method in `impl ProjectDb` (search for `pub async fn list_chunks_for_article` which was added in the ai_coding feature — it's near the end). Add after it:

```rust
    // ── Glossary ──────────────────────────────────────────────────────────────

    /// Return all glossary terms sorted alphabetically.
    pub async fn list_glossary_terms(&self) -> Result<Vec<GlossaryTerm>> {
        let rows = sqlx::query(
            "SELECT id, term, definition, article_ids, created_at, updated_at \
             FROM glossary_term ORDER BY lower(term)",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| GlossaryTerm {
                id: r.get("id"),
                term: r.get("term"),
                definition: r.get("definition"),
                article_ids: r.get("article_ids"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    /// Insert or merge a term. If a term with the same text (case-insensitive)
    /// already exists, the new article_id is merged into its article_ids list
    /// and the definition is updated. Returns the row id.
    pub async fn upsert_glossary_term(
        &self,
        term: &str,
        definition: &str,
        article_id: i64,
    ) -> Result<i64> {
        // Check for existing term (case-insensitive)
        let existing: Option<(i64, String)> = sqlx::query(
            "SELECT id, article_ids FROM glossary_term WHERE lower(term) = lower(?)",
        )
        .bind(term)
        .fetch_optional(&self.pool)
        .await?
        .map(|r| (r.get("id"), r.get("article_ids")));

        if let Some((id, ids_json)) = existing {
            // Merge article_id into the existing JSON array
            let mut ids: Vec<i64> = serde_json::from_str(&ids_json).unwrap_or_default();
            if !ids.contains(&article_id) {
                ids.push(article_id);
            }
            let new_json = serde_json::to_string(&ids).unwrap_or_else(|_| ids_json);
            sqlx::query(
                "UPDATE glossary_term SET definition = ?, article_ids = ?, updated_at = datetime('now') \
                 WHERE id = ?",
            )
            .bind(definition)
            .bind(&new_json)
            .bind(id)
            .execute(&self.pool)
            .await?;
            Ok(id)
        } else {
            let ids_json = serde_json::to_string(&[article_id]).unwrap_or_else(|_| "[]".into());
            let id = sqlx::query(
                "INSERT INTO glossary_term (term, definition, article_ids, created_at, updated_at) \
                 VALUES (?, ?, ?, datetime('now'), datetime('now'))",
            )
            .bind(term)
            .bind(definition)
            .bind(&ids_json)
            .execute(&self.pool)
            .await?
            .last_insert_rowid();
            Ok(id)
        }
    }

    /// Delete a single glossary term by id.
    pub async fn delete_glossary_term(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM glossary_term WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete all glossary terms (used before a full re-extraction).
    pub async fn clear_glossary(&self) -> Result<()> {
        sqlx::query("DELETE FROM glossary_term")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Return the count of glossary terms.
    pub async fn glossary_term_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as n FROM glossary_term")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("n"))
    }

    /// Return all articles that have at least one embedded chunk.
    /// Returns (article_id, article_title, Vec<chunk_text>).
    pub async fn articles_with_chunks(&self) -> Result<Vec<(i64, String, Vec<String>)>> {
        let rows = sqlx::query(
            "SELECT pc.article_id, a.title, pc.text \
             FROM pdf_chunk pc \
             INNER JOIN article a ON a.id = pc.article_id \
             WHERE pc.embedding IS NOT NULL \
             ORDER BY pc.article_id, pc.chunk_index",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut map: std::collections::LinkedHashMap<i64, (String, Vec<String>)> =
            std::collections::LinkedHashMap::new();
        for r in rows {
            let aid: i64 = r.get("article_id");
            let title: String = r.get("title");
            let text: String = r.get("text");
            map.entry(aid).or_insert_with(|| (title, vec![])).1.push(text);
        }
        Ok(map.into_iter().map(|(id, (title, chunks))| (id, title, chunks)).collect())
    }
```

- [ ] **Step 3: Verify Rust compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio/src-tauri
cargo check 2>&1 | grep "^error" | head -20
```

Expected: no errors (LinkedHashMap requires `use std::collections::LinkedHashMap` if not already there — if the compiler complains, replace `LinkedHashMap` with `IndexMap` from the `indexmap` crate already in Cargo.toml, or use a `Vec<(i64, (String, Vec<String>))>` with a manual dedup loop).

If `LinkedHashMap` is not available, replace `articles_with_chunks` body with:
```rust
        let mut result: Vec<(i64, String, Vec<String>)> = vec![];
        for r in rows {
            let aid: i64 = r.get("article_id");
            let title: String = r.get("title");
            let text: String = r.get("text");
            if let Some(entry) = result.iter_mut().find(|(id, _, _)| *id == aid) {
                entry.2.push(text);
            } else {
                result.push((aid, title, vec![text]));
            }
        }
        Ok(result)
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): add GlossaryTerm struct and glossary/articles_with_chunks methods"
```

---

## Task 3: lib.rs — cluster_corpus and extract_glossary commands

**Files:**
- Modify: `src-tauri/src/lib.rs`

Context: All Tauri commands follow the pattern `#[tauri::command] async fn name(args, state) -> Result<T, String>`. The two new commands go at the end of the command section (before `generate_handler!`). Both are registered in `generate_handler!`.

K-means algorithm: compute one centroid per article by averaging its chunk embeddings. Then run Lloyd's algorithm for K iterations. K = min(8, article_count / 2). After convergence, for each cluster collect article titles + first 200 chars of abstracts, call the LLM to produce a 3–5 word label. Emit `clusters:progress` events.

Glossary: for each article, concatenate up to 6 chunks, call the LLM to extract 3–8 domain terms as JSON `[{"term":"...","definition":"..."}]`, upsert into DB. Emit `glossary:progress` events.

- [ ] **Step 1: Add output structs before the `ask_corpus` function**

Find in lib.rs:
```rust
struct AskCitation {
```

Insert before it:
```rust
#[derive(serde::Serialize, Clone)]
struct ClusterProgress {
    phase: String, // "embedding" | "clustering" | "labelling" | "done"
    done: usize,
    total: usize,
}

#[derive(serde::Serialize, Clone)]
struct ClusterArticle {
    id: i64,
    title: String,
}

#[derive(serde::Serialize, Clone)]
struct ClusterNode {
    id: usize,
    label: String,
    articles: Vec<ClusterArticle>,
}

#[derive(serde::Serialize, Clone)]
struct GlossaryProgress {
    done: usize,
    total: usize,
    article_title: String,
}

```

- [ ] **Step 2: Add `cluster_corpus` command**

Find:
```rust
async fn ask_corpus(
```

Insert before it:
```rust
#[tauri::command]
async fn cluster_corpus(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<ClusterNode>, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings.get("lmstudio.base_url").and_then(|v| v.as_str()).unwrap_or("http://127.0.0.1:1234/v1").to_string();
    let api_key = settings.get("lmstudio.api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
    let chat_model = settings.get("lmstudio.chat_model").and_then(|v| v.as_str()).unwrap_or("google/gemma-4-e2b").to_string();

    let _ = app.emit("clusters:progress", ClusterProgress { phase: "embedding".into(), done: 0, total: 0 });

    // Gather per-article centroid embeddings
    let all_chunks = db.all_embedded_chunks().await.map_err(err)?;
    if all_chunks.is_empty() {
        return Err("No embedded chunks. Index and embed PDFs first.".into());
    }

    // Group chunks by article_id, average embeddings to get one centroid per article
    let mut article_chunks: std::collections::HashMap<i64, (String, Vec<Vec<f32>>)> = std::collections::HashMap::new();
    for (_chunk_id, art_id, title, _ci, _text, bytes) in &all_chunks {
        let emb = vector::bytes_to_f32s(bytes);
        article_chunks.entry(*art_id).or_insert_with(|| (title.clone(), vec![])).1.push(emb);
    }

    let articles: Vec<(i64, String, Vec<f32>)> = article_chunks
        .into_iter()
        .map(|(aid, (title, embs))| {
            let dim = embs[0].len();
            let mut centroid = vec![0.0f32; dim];
            for e in &embs {
                for i in 0..dim { centroid[i] += e[i]; }
            }
            let n = embs.len() as f32;
            for v in &mut centroid { *v /= n; }
            (aid, title, centroid)
        })
        .collect();

    let n = articles.len();
    if n < 2 {
        return Err("Need at least 2 articles with embeddings to cluster.".into());
    }

    // K-means: k = min(8, n/2), max 20 iterations
    let k = (n / 2).min(8).max(2);
    let dim = articles[0].2.len();

    // Initialize centroids by picking k evenly-spaced articles
    let mut centroids: Vec<Vec<f32>> = (0..k)
        .map(|i| articles[i * n / k].2.clone())
        .collect();

    let mut assignments = vec![0usize; n];

    for _iter in 0..20 {
        // Assign
        let mut changed = false;
        for (i, (_, _, emb)) in articles.iter().enumerate() {
            let best = (0..k)
                .max_by(|&a, &b| {
                    vector::cosine(emb, &centroids[a])
                        .partial_cmp(&vector::cosine(emb, &centroids[b]))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0);
            if assignments[i] != best { changed = true; }
            assignments[i] = best;
        }
        if !changed { break; }

        // Update centroids
        let mut sums = vec![vec![0.0f32; dim]; k];
        let mut counts = vec![0usize; k];
        for (i, (_, _, emb)) in articles.iter().enumerate() {
            let c = assignments[i];
            for j in 0..dim { sums[c][j] += emb[j]; }
            counts[c] += 1;
        }
        for c in 0..k {
            if counts[c] > 0 {
                let n_c = counts[c] as f32;
                centroids[c] = sums[c].iter().map(|v| v / n_c).collect();
            }
        }
    }

    let _ = app.emit("clusters:progress", ClusterProgress { phase: "labelling".into(), done: 0, total: k });

    // Build cluster groups
    let mut groups: Vec<Vec<(i64, String)>> = vec![vec![]; k];
    for (i, (aid, title, _)) in articles.iter().enumerate() {
        groups[assignments[i]].push((*aid, title.clone()));
    }

    // Remove empty clusters
    let groups: Vec<Vec<(i64, String)>> = groups.into_iter().filter(|g| !g.is_empty()).collect();
    let total_groups = groups.len();

    // Label each cluster via LLM
    let mut nodes = vec![];
    for (idx, group) in groups.into_iter().enumerate() {
        let titles_list = group.iter().enumerate()
            .map(|(i, (_, t))| format!("{}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = format!(
            "These are titles of academic articles in a systematic literature review:\n{}\n\n\
             Give a concise 3–5 word thematic label that best describes what these articles have in common. \
             Reply with ONLY the label, no punctuation, no explanation.",
            titles_list
        );
        let label = match lmstudio::chat(
            &base_url, &chat_model,
            &[("user", prompt.as_str())],
            None, None,
            api_key.as_deref(),
        ).await {
            Ok(r) => r.content.trim().trim_matches('"').to_string(),
            Err(_) => format!("Cluster {}", idx + 1),
        };
        let _ = app.emit("clusters:progress", ClusterProgress { phase: "labelling".into(), done: idx + 1, total: total_groups });
        nodes.push(ClusterNode {
            id: idx,
            label,
            articles: group.into_iter().map(|(id, title)| ClusterArticle { id, title }).collect(),
        });
    }

    let _ = app.emit("clusters:progress", ClusterProgress { phase: "done".into(), done: total_groups, total: total_groups });
    Ok(nodes)
}

```

- [ ] **Step 3: Add `extract_glossary` command**

Find:
```rust
#[tauri::command]
async fn cluster_corpus(
```

Insert before it:
```rust
#[tauri::command]
async fn extract_glossary(app: AppHandle, state: State<'_, AppState>) -> Result<usize, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings.get("lmstudio.base_url").and_then(|v| v.as_str()).unwrap_or("http://127.0.0.1:1234/v1").to_string();
    let api_key = settings.get("lmstudio.api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
    let chat_model = settings.get("lmstudio.chat_model").and_then(|v| v.as_str()).unwrap_or("google/gemma-4-e2b").to_string();

    db.clear_glossary().await.map_err(err)?;

    let articles = db.articles_with_chunks().await.map_err(err)?;
    let total = articles.len();
    if total == 0 {
        return Err("No indexed articles. Index and embed PDFs first.".into());
    }

    for (i, (article_id, title, chunks)) in articles.iter().enumerate() {
        let _ = app.emit("glossary:progress", GlossaryProgress {
            done: i,
            total,
            article_title: title.clone(),
        });

        // Concatenate up to 6 chunks (≈ 6000 chars)
        let context: String = chunks.iter().take(6).cloned().collect::<Vec<_>>().join("\n\n");
        let context = if context.len() > 6000 { context[..6000].to_string() } else { context };

        let prompt = format!(
            "You are extracting a domain glossary from an academic article.\n\
             Article title: {}\n\n\
             Article text (excerpt):\n{}\n\n\
             Extract 3 to 8 domain-specific technical terms or constructs that are important in this article. \
             For each term provide a concise one-sentence definition as used in this article.\n\
             Reply with a JSON array only, no prose before or after:\n\
             [{\"term\":\"Term name\",\"definition\":\"Definition sentence.\"}]",
            title, context
        );

        let response = match lmstudio::chat(
            &base_url, &chat_model,
            &[("user", prompt.as_str())],
            None, None,
            api_key.as_deref(),
        ).await {
            Ok(r) => r.content,
            Err(_) => continue,
        };

        // Extract JSON array from response
        let json_start = response.find('[');
        let json_end = response.rfind(']');
        if let (Some(s), Some(e)) = (json_start, json_end) {
            let json_str = &response[s..=e];
            if let Ok(terms) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
                for t in terms {
                    let term = t.get("term").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    let def = t.get("definition").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if !term.is_empty() && !def.is_empty() {
                        let _ = db.upsert_glossary_term(&term, &def, *article_id).await;
                    }
                }
            }
        }
    }

    let _ = app.emit("glossary:progress", GlossaryProgress {
        done: total,
        total,
        article_title: String::new(),
    });

    let count = db.glossary_term_count().await.map_err(err)?;
    Ok(count as usize)
}

```

- [ ] **Step 4: Add `list_glossary_terms` and `delete_glossary_term` commands**

Find:
```rust
#[tauri::command]
async fn extract_glossary(
```

Insert before it:
```rust
#[tauri::command]
async fn list_glossary_terms(state: State<'_, AppState>) -> Result<Vec<db::GlossaryTerm>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_glossary_terms().await.map_err(err)
}

#[tauri::command]
async fn delete_glossary_term(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.delete_glossary_term(id).await.map_err(err)
}

```

- [ ] **Step 5: Register all four new commands in `generate_handler!`**

Find the `generate_handler![` macro (it lists all commands). Add to it:
```
cluster_corpus,
extract_glossary,
list_glossary_terms,
delete_glossary_term,
```

- [ ] **Step 6: Verify Rust compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio/src-tauri
cargo check 2>&1 | grep "^error" | head -20
```

Expected: no errors. Common issues:
- `lmstudio::chat` signature: check it matches `(base_url, model, messages, system_prompt, max_tokens, api_key)` — look at the existing `ask_corpus` call for the exact signature. The `messages` parameter is `&[(&str, &str)]`.
- If `lmstudio::chat` returns a struct with a `.content` field, use that. If it returns a `String`, use it directly.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(backend): add cluster_corpus, extract_glossary, list/delete glossary commands"
```

---

## Task 4: api.ts — TypeScript types and bridge functions

**Files:**
- Modify: `src/lib/api.ts`

- [ ] **Step 1: Add interfaces before `PdfRow`**

Find:
```typescript
export interface PdfRow {
```

Insert before it:
```typescript
export interface ClusterArticle {
  id: number;
  title: string;
}

export interface ClusterNode {
  id: number;
  label: string;
  articles: ClusterArticle[];
}

export interface ClusterProgress {
  phase: "embedding" | "clustering" | "labelling" | "done";
  done: number;
  total: number;
}

export interface GlossaryTerm {
  id: number;
  term: string;
  definition: string;
  /** JSON string like "[1,3,7]" */
  article_ids: string;
  created_at: string;
  updated_at: string;
}

export interface GlossaryProgress {
  done: number;
  total: number;
  article_title: string;
}

```

- [ ] **Step 2: Add bridge functions after `cancelAiCoding`**

Find:
```typescript
  cancelAiCoding() { return invoke<void>("cancel_ai_coding"); },
```

Add after it:
```typescript
  clusterCorpus() { return invoke<ClusterNode[]>("cluster_corpus"); },
  extractGlossary() { return invoke<number>("extract_glossary"); },
  listGlossaryTerms() { return invoke<GlossaryTerm[]>("list_glossary_terms"); },
  deleteGlossaryTerm(id: number) { return invoke<void>("delete_glossary_term", { id }); },
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio
npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): add cluster and glossary bridge types and functions"
```

---

## Task 5: Install @xyflow/react

**Files:**
- Modify: `package.json` (via npm)

- [ ] **Step 1: Install the package**

```bash
cd /Users/david/Documents/GitHub/Fangio
npm install @xyflow/react
```

- [ ] **Step 2: Verify it installed**

```bash
grep "@xyflow/react" package.json
```

Expected: a line like `"@xyflow/react": "^12.x.x"`

- [ ] **Step 3: Commit**

```bash
git add package.json package-lock.json
git commit -m "deps: add @xyflow/react for mind map visualisation"
```

---

## Task 6: Extract ChatTab from RagChatView

**Files:**
- Create: `src/views/rag/ChatTab.tsx`
- Modify: `src/views/RagChatView.tsx`

Context: `RagChatView.tsx` currently is the entire chat UI (536 lines). We need to split it into a tab shell + the existing chat panel. The chat logic and JSX moves into `ChatTab.tsx`. `RagChatView.tsx` becomes the tab shell with three tabs.

- [ ] **Step 1: Create `src/views/rag/` directory and `ChatTab.tsx`**

`ChatTab.tsx` is the entire current content of `RagChatView.tsx` except the outer route/back navigation wrapper. It should be a component that accepts these props:

```typescript
interface Props {
  onOpenSettings: () => void;
}
```

Copy the entire body of `RagChatView` (all state, effects, handlers, JSX) into `ChatTab`. The outer `<div className="flex flex-col h-full ...">` and everything inside it becomes the `ChatTab` return. Move the helper components `ThinkingDots` and `CitationsBlock` into the same file.

```typescript
// src/views/rag/ChatTab.tsx
import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { motion, AnimatePresence } from "framer-motion";
import { Markdown } from "../../components/Markdown";
import {
  BookOpen,
  ChevronDown,
  ChevronUp,
  Database,
  Info,
  Loader2,
  MessageSquare,
  RefreshCw,
  Send,
  Settings as SettingsIcon,
  Sparkles,
} from "lucide-react";
import { api, type AskCitation, type BuildRagProgress, type RagStatus } from "../../lib/api";
import { cn } from "../../lib/utils";

interface Props {
  onOpenSettings: () => void;
}
```

Then paste the full existing body (state, effects, JSX) — adjusting import paths from `../lib/api` to `../../lib/api` and `../components/Markdown` to `../../components/Markdown`.

- [ ] **Step 2: Rewrite `RagChatView.tsx` as a three-tab shell**

Replace the entire content of `RagChatView.tsx` with:

```typescript
import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ArrowLeft, MessageSquare, Network, BookOpen } from "lucide-react";
import { ChatTab } from "./rag/ChatTab";
import { ClustersTab } from "./rag/ClustersTab";
import { GlossaryTab } from "./rag/GlossaryTab";

interface Props {
  onBack: () => void;
  onOpenSettings: () => void;
}

type Tab = "chat" | "clusters" | "glossary";

const TABS: { id: Tab; label: string; Icon: React.ElementType }[] = [
  { id: "chat",     label: "Chat",     Icon: MessageSquare },
  { id: "clusters", label: "Clusters", Icon: Network },
  { id: "glossary", label: "Glossary", Icon: BookOpen },
];

export function RagChatView({ onBack, onOpenSettings }: Props) {
  const [tab, setTab] = useState<Tab>("chat");

  return (
    <div className="flex flex-col h-screen bg-[var(--color-paper-50)]">
      {/* ── Top bar ── */}
      <div className="flex items-center gap-3 px-5 h-14 border-b border-[var(--color-paper-200)] bg-white shrink-0">
        <button
          onClick={onBack}
          className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-100)] text-[var(--color-paper-500)] transition-paper"
        >
          <ArrowLeft size={16} strokeWidth={2} />
        </button>
        <span className="text-[13px] font-semibold text-[var(--color-ink-800)]">Ask the Literature</span>

        {/* Tab strip */}
        <div className="flex items-center gap-1 ml-4 bg-[var(--color-paper-100)] rounded-[var(--radius-sm)] p-0.5">
          {TABS.map(({ id, label, Icon }) => (
            <button
              key={id}
              onClick={() => setTab(id)}
              className={cn(
                "flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-xs)] text-[12px] font-medium transition-paper",
                tab === id
                  ? "bg-white text-[var(--color-ink-800)] shadow-sm"
                  : "text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)]",
              )}
            >
              <Icon size={13} strokeWidth={tab === id ? 2 : 1.75} />
              {label}
            </button>
          ))}
        </div>
      </div>

      {/* ── Tab content ── */}
      <div className="flex-1 min-h-0">
        <AnimatePresence mode="wait">
          {tab === "chat" && (
            <motion.div key="chat" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.15 }} className="h-full">
              <ChatTab onOpenSettings={onOpenSettings} />
            </motion.div>
          )}
          {tab === "clusters" && (
            <motion.div key="clusters" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.15 }} className="h-full">
              <ClustersTab />
            </motion.div>
          )}
          {tab === "glossary" && (
            <motion.div key="glossary" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.15 }} className="h-full">
              <GlossaryTab />
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
```

Note: `cn` needs to be imported: `import { cn } from "../lib/utils";`

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio
npx tsc --noEmit 2>&1 | head -30
```

Expected: errors only about `ClustersTab` and `GlossaryTab` not existing yet — those are acceptable at this stage since the stubs will be created next.

- [ ] **Step 4: Create stub files so TypeScript is happy**

Create `src/views/rag/ClustersTab.tsx`:
```typescript
export function ClustersTab() {
  return <div className="p-6 text-[var(--color-paper-500)]">Clusters coming soon</div>;
}
```

Create `src/views/rag/GlossaryTab.tsx`:
```typescript
export function GlossaryTab() {
  return <div className="p-6 text-[var(--color-paper-500)]">Glossary coming soon</div>;
}
```

- [ ] **Step 5: Verify TypeScript compiles cleanly**

```bash
npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add src/views/RagChatView.tsx src/views/rag/ChatTab.tsx src/views/rag/ClustersTab.tsx src/views/rag/GlossaryTab.tsx
git commit -m "refactor(rag): extract ChatTab, add three-tab shell with stub Clusters and Glossary"
```

---

## Task 7: ClustersTab — K-means mind map

**Files:**
- Modify: `src/views/rag/ClustersTab.tsx`

Context: Replace the stub. This tab calls `api.clusterCorpus()` which runs K-means + LLM labelling on the backend and returns `ClusterNode[]`. The mind map is rendered as a tree using `@xyflow/react`: one root node "Your Literature", then one node per cluster as children, then article nodes as leaves of each cluster. Use `dagre` layout (built-in with react-flow) or a manual tree layout. Since the `@xyflow/react` dagre integration may need `@dagrejs/dagre` package — check if it's installed; if not, use a simple manual radial/tree layout computed in JS.

The simpler approach that avoids dagre: manually position nodes in a top-down tree. Root at (0, 0). Clusters at y=120, evenly spaced by x. Articles at y=240, grouped under their cluster.

```typescript
import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  ReactFlow,
  Background,
  Controls,
  type Node,
  type Edge,
  useNodesState,
  useEdgesState,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Loader2, Network, RefreshCw } from "lucide-react";
import { api, type ClusterNode, type ClusterProgress } from "../../lib/api";

// Build react-flow nodes and edges from cluster data
function buildGraph(clusters: ClusterNode[]): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];

  // Root node
  nodes.push({
    id: "root",
    type: "default",
    position: { x: 0, y: 0 },
    data: { label: "Your Literature" },
    style: {
      background: "var(--color-ink-800)",
      color: "white",
      border: "none",
      borderRadius: "var(--radius-sm)",
      fontSize: "13px",
      fontWeight: 600,
      padding: "8px 16px",
    },
  });

  const clusterSpacing = Math.max(220, (clusters.reduce((m, c) => Math.max(m, c.articles.length), 0)) * 160);

  clusters.forEach((cluster, ci) => {
    const clusterX = (ci - (clusters.length - 1) / 2) * clusterSpacing;
    const clusterId = `cluster-${cluster.id}`;

    nodes.push({
      id: clusterId,
      type: "default",
      position: { x: clusterX, y: 140 },
      data: { label: cluster.label },
      style: {
        background: "var(--color-accent-50)",
        border: "1px solid var(--color-accent-300)",
        borderRadius: "var(--radius-sm)",
        fontSize: "12px",
        fontWeight: 600,
        color: "var(--color-accent-800)",
        padding: "6px 12px",
        maxWidth: 180,
        textAlign: "center",
      },
    });

    edges.push({
      id: `root-${clusterId}`,
      source: "root",
      target: clusterId,
      style: { stroke: "var(--color-paper-300)", strokeWidth: 1.5 },
    });

    cluster.articles.forEach((article, ai) => {
      const articleId = `article-${article.id}`;
      const articleX = clusterX + (ai - (cluster.articles.length - 1) / 2) * 160;

      nodes.push({
        id: articleId,
        type: "default",
        position: { x: articleX, y: 300 },
        data: { label: article.title.length > 50 ? article.title.slice(0, 50) + "…" : article.title },
        style: {
          background: "white",
          border: "1px solid var(--color-paper-200)",
          borderRadius: "var(--radius-xs)",
          fontSize: "10px",
          color: "var(--color-ink-700)",
          padding: "4px 8px",
          maxWidth: 150,
          textAlign: "center",
        },
      });

      edges.push({
        id: `${clusterId}-${articleId}`,
        source: clusterId,
        target: articleId,
        style: { stroke: "var(--color-paper-200)", strokeWidth: 1 },
      });
    });
  });

  return { nodes, edges };
}

export function ClustersTab() {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [loading, setLoading] = useState(false);
  const [progress, setProgress] = useState<ClusterProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [hasData, setHasData] = useState(false);

  const run = useCallback(async () => {
    setLoading(true);
    setError(null);
    setProgress(null);

    const unlisten = await listen<ClusterProgress>("clusters:progress", (e) => {
      setProgress(e.payload);
    });

    try {
      const clusters = await api.clusterCorpus();
      const { nodes: n, edges: e } = buildGraph(clusters);
      setNodes(n);
      setEdges(e);
      setHasData(true);
    } catch (e: any) {
      setError(e?.message ?? String(e));
    } finally {
      unlisten();
      setLoading(false);
    }
  }, [setNodes, setEdges]);

  if (!hasData && !loading) {
    return (
      <div className="h-full flex flex-col items-center justify-center gap-4 p-8 text-center">
        <Network size={36} strokeWidth={1.25} className="text-[var(--color-paper-300)]" />
        <div>
          <p className="text-[14px] font-semibold text-[var(--color-ink-800)]">Cluster your literature</p>
          <p className="text-[12px] text-[var(--color-paper-500)] mt-1 max-w-xs">
            Groups articles by semantic similarity using K-means on their embeddings, then labels each cluster with the LLM.
            Requires indexed and embedded PDFs.
          </p>
        </div>
        {error && <p className="text-[12px] text-[var(--color-error-600)]">{error}</p>}
        <button
          onClick={run}
          className="flex items-center gap-1.5 px-4 py-2 rounded-[var(--radius-sm)] bg-[var(--color-ink-800)] text-white text-[12px] font-semibold hover:bg-[var(--color-ink-700)] transition-paper"
        >
          <Network size={13} strokeWidth={2} /> Build clusters
        </button>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="h-full flex flex-col items-center justify-center gap-3">
        <Loader2 size={24} className="animate-spin text-[var(--color-accent-500)]" />
        <p className="text-[13px] text-[var(--color-paper-600)]">
          {progress
            ? progress.phase === "labelling"
              ? `Labelling cluster ${progress.done + 1} of ${progress.total}…`
              : `${progress.phase}…`
            : "Starting…"}
        </p>
      </div>
    );
  }

  return (
    <div className="h-full relative">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        fitView
        fitViewOptions={{ padding: 0.15 }}
        minZoom={0.2}
        maxZoom={2}
      >
        <Background color="var(--color-paper-200)" gap={20} />
        <Controls />
      </ReactFlow>
      <button
        onClick={run}
        className="absolute top-3 right-3 z-10 flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] bg-white border border-[var(--color-paper-300)] text-[11px] font-medium text-[var(--color-paper-600)] hover:text-[var(--color-ink-800)] shadow-sm transition-paper"
      >
        <RefreshCw size={11} strokeWidth={2} /> Rebuild
      </button>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio
npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors. If `@xyflow/react` types are missing, run `npm install` first.

- [ ] **Step 3: Commit**

```bash
git add src/views/rag/ClustersTab.tsx
git commit -m "feat(rag): add ClustersTab with K-means mind map using react-flow"
```

---

## Task 8: GlossaryTab — searchable term list

**Files:**
- Modify: `src/views/rag/GlossaryTab.tsx`

Context: Replace the stub. The tab loads glossary terms from the DB, shows a searchable list, and has an "Extract glossary" button that calls `api.extractGlossary()` (which clears + re-extracts). Each term shows the term name, definition, and a count badge for how many articles use it. Clicking a term shows the full definition in a tooltip/expanded row. Terms can be deleted individually.

```typescript
import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { BookOpen, Loader2, RefreshCw, Search, Trash2, X } from "lucide-react";
import { api, type GlossaryProgress, type GlossaryTerm } from "../../lib/api";

export function GlossaryTab() {
  const [terms, setTerms] = useState<GlossaryTerm[]>([]);
  const [loading, setLoading] = useState(false);
  const [extracting, setExtracting] = useState(false);
  const [progress, setProgress] = useState<GlossaryProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [expandedId, setExpandedId] = useState<number | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const t = await api.listGlossaryTerms();
      setTerms(t);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const extract = useCallback(async () => {
    setExtracting(true);
    setError(null);
    setProgress(null);

    const unlisten = await listen<GlossaryProgress>("glossary:progress", (e) => {
      setProgress(e.payload);
    });

    try {
      await api.extractGlossary();
      await load();
    } catch (e: any) {
      setError(e?.message ?? String(e));
    } finally {
      unlisten();
      setExtracting(false);
      setProgress(null);
    }
  }, [load]);

  const deleteTerm = async (id: number) => {
    await api.deleteGlossaryTerm(id);
    setTerms((prev) => prev.filter((t) => t.id !== id));
    if (expandedId === id) setExpandedId(null);
  };

  const filtered = terms.filter((t) =>
    t.term.toLowerCase().includes(query.toLowerCase()) ||
    t.definition.toLowerCase().includes(query.toLowerCase()),
  );

  return (
    <div className="h-full flex flex-col">
      {/* Toolbar */}
      <div className="flex items-center gap-3 px-5 py-3 border-b border-[var(--color-paper-200)] bg-white shrink-0">
        <div className="flex-1 relative">
          <Search size={13} strokeWidth={1.75} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--color-paper-400)]" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search terms…"
            className="w-full pl-8 pr-3 py-1.5 text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-400)] transition-paper"
          />
          {query && (
            <button onClick={() => setQuery("")} className="absolute right-2.5 top-1/2 -translate-y-1/2 text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)]">
              <X size={11} strokeWidth={2} />
            </button>
          )}
        </div>
        <span className="text-[11px] text-[var(--color-paper-400)] shrink-0">
          {terms.length} term{terms.length !== 1 ? "s" : ""}
        </span>
        <button
          onClick={extract}
          disabled={extracting}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] bg-[var(--color-ink-800)] text-white text-[11px] font-semibold hover:bg-[var(--color-ink-700)] disabled:opacity-50 transition-paper shrink-0"
        >
          {extracting
            ? <Loader2 size={11} className="animate-spin" />
            : <RefreshCw size={11} strokeWidth={2} />
          }
          {extracting ? "Extracting…" : terms.length > 0 ? "Re-extract" : "Extract glossary"}
        </button>
      </div>

      {/* Progress bar during extraction */}
      {extracting && progress && (
        <div className="px-5 py-2 bg-[var(--color-accent-50)] border-b border-[var(--color-accent-100)] shrink-0">
          <div className="flex justify-between text-[11px] text-[var(--color-paper-500)] mb-1">
            <span className="truncate max-w-xs">{progress.article_title || "Starting…"}</span>
            <span className="font-mono">{progress.done}/{progress.total}</span>
          </div>
          <div className="h-1 rounded-full bg-[var(--color-paper-200)] overflow-hidden">
            <div
              className="h-full rounded-full bg-[var(--color-accent-500)] transition-all duration-300"
              style={{ width: progress.total > 0 ? `${Math.round((progress.done / progress.total) * 100)}%` : "0%" }}
            />
          </div>
        </div>
      )}

      {/* Error */}
      {error && (
        <p className="px-5 py-2 text-[12px] text-[var(--color-error-600)] bg-[var(--color-error-50)] border-b border-[var(--color-error-100)] shrink-0">
          {error}
        </p>
      )}

      {/* Term list */}
      <div className="flex-1 overflow-y-auto">
        {loading && (
          <div className="flex items-center justify-center py-16">
            <Loader2 size={20} className="animate-spin text-[var(--color-paper-400)]" />
          </div>
        )}

        {!loading && terms.length === 0 && (
          <div className="flex flex-col items-center justify-center gap-3 py-16 text-center px-8">
            <BookOpen size={32} strokeWidth={1.25} className="text-[var(--color-paper-300)]" />
            <p className="text-[13px] font-semibold text-[var(--color-ink-800)]">No glossary yet</p>
            <p className="text-[12px] text-[var(--color-paper-500)] max-w-xs">
              Click "Extract glossary" to have the LLM identify domain-specific terms and definitions from your indexed articles.
            </p>
          </div>
        )}

        {!loading && filtered.length === 0 && terms.length > 0 && (
          <p className="px-5 py-8 text-center text-[12px] text-[var(--color-paper-400)]">
            No terms match "{query}"
          </p>
        )}

        {!loading && filtered.length > 0 && (
          <div className="divide-y divide-[var(--color-paper-100)]">
            {filtered.map((term) => {
              const articleCount = (() => {
                try { return JSON.parse(term.article_ids).length; } catch { return 0; }
              })();
              const isExpanded = expandedId === term.id;

              return (
                <div key={term.id} className="group px-5 py-3 hover:bg-[var(--color-paper-50)] transition-paper">
                  <div className="flex items-start gap-2">
                    <div
                      className="flex-1 min-w-0 cursor-pointer"
                      onClick={() => setExpandedId(isExpanded ? null : term.id)}
                    >
                      <div className="flex items-center gap-2 mb-0.5">
                        <span className="text-[13px] font-semibold text-[var(--color-ink-900)]">{term.term}</span>
                        <span className="text-[10px] font-medium px-1.5 py-0.5 rounded-full bg-[var(--color-paper-100)] text-[var(--color-paper-500)]">
                          {articleCount} article{articleCount !== 1 ? "s" : ""}
                        </span>
                      </div>
                      <p className={`text-[12px] text-[var(--color-paper-600)] leading-relaxed ${isExpanded ? "" : "line-clamp-2"}`}>
                        {term.definition}
                      </p>
                    </div>
                    <button
                      onClick={() => deleteTerm(term.id)}
                      className="p-1 rounded-[var(--radius-xs)] text-[var(--color-paper-300)] hover:text-[var(--color-error-500)] opacity-0 group-hover:opacity-100 transition-paper shrink-0 mt-0.5"
                    >
                      <Trash2 size={12} strokeWidth={1.75} />
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio
npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/views/rag/GlossaryTab.tsx
git commit -m "feat(rag): add GlossaryTab with extraction progress and searchable list"
```

---

## Task 9: Commit inclusion note fix

**Files:**
- Modify: `src/views/arranging/CodingPanel.tsx` (already done — the ChevronDown was removed from the "Inclusion note" button)

- [ ] **Step 1: Verify the fix is in place**

```bash
grep -n "Inclusion note" /Users/david/Documents/GitHub/Fangio/src/views/arranging/CodingPanel.tsx
```

Expected: the button should not have `<ChevronDown` on the same or adjacent lines.

- [ ] **Step 2: Commit**

```bash
git add src/views/arranging/CodingPanel.tsx
git commit -m "fix(coding-panel): remove chevron arrow from inclusion note button"
```

---

## Self-Review

**Spec coverage:**
- ✅ Three tabs in the "Ask the Literature" view → Tasks 6, 7, 8
- ✅ Chat tab (existing functionality preserved) → Task 6
- ✅ K-means clustering on existing embeddings → Task 3 (`cluster_corpus`)
- ✅ LLM cluster labelling → Task 3
- ✅ Mind map visualisation → Task 7 (react-flow)
- ✅ Glossary table → Task 1
- ✅ LLM term extraction per article → Task 3 (`extract_glossary`)
- ✅ Deduplication / upsert of terms → Task 2 (`upsert_glossary_term`)
- ✅ Searchable glossary list → Task 8
- ✅ Delete individual term → Task 2 + Task 8
- ✅ Re-extract glossary → Task 3 (clears + re-runs)
- ✅ Inclusion note fix (no arrow) → Task 9

**Placeholder scan:** None found.

**Type consistency:**
- `ClusterNode.id: usize` in Rust → `number` in TS ✅
- `GlossaryTerm.article_ids` is a JSON string in both Rust and TS ✅
- `GlossaryProgress` event name `"glossary:progress"` matches in lib.rs and GlossaryTab ✅
- `ClusterProgress` event name `"clusters:progress"` matches in lib.rs and ClustersTab ✅
- `lmstudio::chat` called with `&[("user", ...)]` — matches existing usage in `ask_corpus` at lib.rs line ~2160 ✅
