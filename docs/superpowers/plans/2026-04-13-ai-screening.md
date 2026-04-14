# AI Screening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add AI-assisted title/abstract screening to Stage 2, with a three-tab panel (Pending / Included / Excluded), a guided criteria modal, and an article-by-article progress run that auto-applies high-confidence decisions and flags uncertain ones for manual review.

**Architecture:** New DB columns (`ai_rationale`, `ai_confidence`) on `article`; new Rust commands for tab-aware pagination and the AI run (emitting `screen:progress` events like `rag:progress`); new `AiScreeningModal` React component; `ScreeningPanel` refactored to three independent tabs.

**Tech Stack:** Rust/SQLx (Tauri v2), React + TypeScript, Framer Motion, Tailwind CSS, LM Studio via existing `lmstudio::chat`.

---

## File Map

| File | Action | Purpose |
|---|---|---|
| `src-tauri/migrations/0012_ai_screening.sql` | **Create** | Add `ai_rationale TEXT`, `ai_confidence REAL` to `article` |
| `src-tauri/src/db.rs` | **Modify** | Add fields to `ArticleRow`; update all SELECT queries; add `list_articles_for_tab`, `count_articles_for_tab`, `apply_ai_outcome`; update `clear_exclusion` |
| `src-tauri/src/suggest.rs` | **Modify** | Add `screen_article` function |
| `src-tauri/src/lib.rs` | **Modify** | Add `run_ai_screening`, `list_screening_tab`, `count_screening_tab` commands; register them |
| `src/lib/useAppState.ts` | **Modify** | Add `"screening.confidence_threshold": number` to `AppSettings` |
| `src/views/Settings.tsx` | **Modify** | Add confidence threshold number input under AI models section |
| `src/lib/api.ts` | **Modify** | Update `ArticleRow`; add `listScreeningTab`, `countScreeningTab`, `runAiScreening`; add `AiScreeningProgress`, `AiScreeningResult` types |
| `src/components/AiScreeningModal.tsx` | **Create** | Three-step modal: criteria checklist → extra criteria prompt → progress run |
| `src/views/arranging/ScreeningPanel.tsx` | **Modify** | Three-tab layout (Pending/Included/Excluded), independent pagination per tab, AI button, integrate modal |

---

### Task 1: DB Migration

**Files:**
- Create: `src-tauri/migrations/0012_ai_screening.sql`

- [ ] **Step 1: Create the migration file**

```sql
-- 0012_ai_screening.sql
-- AI screening: add per-article AI rationale and confidence score.
ALTER TABLE article ADD COLUMN ai_rationale TEXT;
ALTER TABLE article ADD COLUMN ai_confidence REAL;
```

- [ ] **Step 2: Verify migration runs**

```bash
cd /Users/david/Documents/GitHub/Fangio
cargo build -p reeves 2>&1 | grep -E "error|warning: unused|migrate"
```

Expected: clean build (sqlx runs migrations on startup; errors here usually mean SQL syntax problems).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/migrations/0012_ai_screening.sql
git commit -m "feat(db): add ai_rationale and ai_confidence columns to article"
```

---

### Task 2: Update ArticleRow Struct and All SELECT Queries

**Files:**
- Modify: `src-tauri/src/db.rs`

Every place that builds an `ArticleRow` must select and map the two new columns. There are six such sites.

- [ ] **Step 1: Add fields to the struct (db.rs ~line 62)**

Replace the `ArticleRow` struct:

```rust
pub struct ArticleRow {
    pub id: i64,
    pub title: String,
    pub authors: Option<String>,
    pub year: Option<i32>,
    pub journal: Option<String>,
    pub abstract_text: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
    pub source_databases: Option<String>,
    pub status: String,
    pub dedup_group_id: Option<i64>,
    pub exclusion_criterion: Option<i64>,
    pub created_at: String,
    /// Set only when the user explicitly screened the article (include or exclude).
    /// NULL for articles auto-included via PDF attachment.
    pub screened_at: Option<String>,
    /// Rationale recorded by the AI screener (null if human-screened).
    pub ai_rationale: Option<String>,
    /// Confidence score 0–1 from the AI screener (null if human-screened).
    pub ai_confidence: Option<f64>,
}
```

- [ ] **Step 2: Update `list_articles` SELECT (~line 473)**

```rust
pub async fn list_articles(&self) -> Result<Vec<ArticleRow>> {
    let rows = sqlx::query(
        "SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
         status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
         ai_rationale, ai_confidence \
         FROM article ORDER BY created_at DESC",
    )
    .fetch_all(&self.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ArticleRow {
            id: r.get("id"),
            title: r.get("title"),
            authors: r.get("authors"),
            year: r.get("year"),
            journal: r.get("journal"),
            abstract_text: r.get("abstract"),
            doi: r.get("doi"),
            url: r.get("url"),
            source_databases: r.get("source_databases"),
            status: r.get("status"),
            dedup_group_id: r.get("dedup_group_id"),
            exclusion_criterion: r.get("exclusion_criterion"),
            created_at: r.get("created_at"),
            screened_at: r.get("screened_at"),
            ai_rationale: r.get("ai_rationale"),
            ai_confidence: r.get("ai_confidence"),
        })
        .collect())
}
```

- [ ] **Step 3: Update `list_articles_for_coding` SELECT (~line 725)**

Change the SQL string and add the two new fields to the mapping:

```rust
"SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
 status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
 ai_rationale, ai_confidence \
 FROM article WHERE status IN ('retrieved', 'included') ORDER BY id LIMIT ? OFFSET ?",
```

Add to the `.map(|r| ArticleRow { ... })` block:
```rust
ai_rationale: r.get("ai_rationale"),
ai_confidence: r.get("ai_confidence"),
```

- [ ] **Step 4: Update `list_articles_for_screening` SELECT (~line 892)**

```rust
"SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
 status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
 ai_rationale, ai_confidence \
 FROM article \
 WHERE status IN ('retrieved', 'included') \
 ORDER BY (screened_at IS NOT NULL AND status = 'included'), id LIMIT ? OFFSET ?",
```

Add to the `.map` block:
```rust
ai_rationale: r.get("ai_rationale"),
ai_confidence: r.get("ai_confidence"),
```

- [ ] **Step 5: Update `list_included_articles_full` (~line 1376) and `list_excluded_articles_full` (~line 1404)**

For `list_included_articles_full`:
```rust
"SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
 status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
 ai_rationale, ai_confidence \
 FROM article WHERE status = 'included' ORDER BY id",
```

For `list_excluded_articles_full`:
```rust
"SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
 status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
 ai_rationale, ai_confidence \
 FROM article WHERE status = 'excluded' ORDER BY id",
```

Add to both `.map` blocks:
```rust
ai_rationale: r.get("ai_rationale"),
ai_confidence: r.get("ai_confidence"),
```

- [ ] **Step 6: Build to confirm no compile errors**

```bash
cargo build -p reeves 2>&1 | grep "^error"
```

Expected: no output (no errors).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): expose ai_rationale and ai_confidence on ArticleRow"
```

---

### Task 3: New DB Methods

**Files:**
- Modify: `src-tauri/src/db.rs`

Add four methods: `list_articles_for_tab`, `count_articles_for_tab`, `apply_ai_outcome`, and update `clear_exclusion` to wipe AI fields on revert.

- [ ] **Step 1: Add `list_articles_for_tab` after `list_articles_for_screening`**

```rust
/// Tab-aware paginated article list for the three-tab screening panel.
/// tab: "pending" | "included" | "excluded"
pub async fn list_articles_for_tab(
    &self,
    tab: &str,
    offset: i64,
    limit: i64,
) -> Result<Vec<ArticleRow>> {
    let sql = match tab {
        "included" =>
            "SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
             status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
             ai_rationale, ai_confidence \
             FROM article WHERE status = 'included' \
             ORDER BY screened_at DESC NULLS LAST, id LIMIT ? OFFSET ?",
        "excluded" =>
            "SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
             status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
             ai_rationale, ai_confidence \
             FROM article WHERE status = 'excluded' \
             AND COALESCE(exclusion_criterion, 0) != 1 \
             ORDER BY screened_at DESC NULLS LAST, id LIMIT ? OFFSET ?",
        _ => // "pending"
            "SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
             status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
             ai_rationale, ai_confidence \
             FROM article WHERE status = 'retrieved' \
             ORDER BY (ai_confidence IS NOT NULL) DESC, id ASC LIMIT ? OFFSET ?",
    };
    let rows = sqlx::query(sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| ArticleRow {
            id: r.get("id"),
            title: r.get("title"),
            authors: r.get("authors"),
            year: r.get("year"),
            journal: r.get("journal"),
            abstract_text: r.get("abstract"),
            doi: r.get("doi"),
            url: r.get("url"),
            source_databases: r.get("source_databases"),
            status: r.get("status"),
            dedup_group_id: r.get("dedup_group_id"),
            exclusion_criterion: r.get("exclusion_criterion"),
            created_at: r.get("created_at"),
            screened_at: r.get("screened_at"),
            ai_rationale: r.get("ai_rationale"),
            ai_confidence: r.get("ai_confidence"),
        })
        .collect())
}
```

- [ ] **Step 2: Add `count_articles_for_tab` right after**

```rust
pub async fn count_articles_for_tab(&self, tab: &str) -> Result<i64> {
    let n: i64 = match tab {
        "included" => sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'included'",
        )
        .fetch_one(&self.pool)
        .await?,
        "excluded" => sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'excluded' \
             AND COALESCE(exclusion_criterion, 0) != 1",
        )
        .fetch_one(&self.pool)
        .await?,
        _ => sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'retrieved'",
        )
        .fetch_one(&self.pool)
        .await?,
    };
    Ok(n)
}
```

- [ ] **Step 3: Add `apply_ai_outcome` right after**

```rust
/// Record an AI screening decision on an article.
/// decision: "include" | "exclude" | "uncertain"
/// For "uncertain" the status stays 'retrieved'; ai_rationale and ai_confidence are stored.
pub async fn apply_ai_outcome(
    &self,
    article_id: i64,
    decision: &str,
    criterion: Option<i64>,
    rationale: &str,
    confidence: f64,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    match decision {
        "include" => {
            sqlx::query(
                "UPDATE article SET status = 'included', screened_at = ?, \
                 ai_rationale = ?, ai_confidence = ?, updated_at = ? WHERE id = ?",
            )
            .bind(&now)
            .bind(rationale)
            .bind(confidence)
            .bind(&now)
            .bind(article_id)
            .execute(&self.pool)
            .await?;
        }
        "exclude" => {
            sqlx::query(
                "UPDATE article SET status = 'excluded', exclusion_criterion = ?, \
                 exclusion_rationale = ?, screened_at = ?, \
                 ai_rationale = ?, ai_confidence = ?, updated_at = ? WHERE id = ?",
            )
            .bind(criterion.unwrap_or(8)) // default EC8: not relevant to domain
            .bind(rationale)
            .bind(&now)
            .bind(rationale)
            .bind(confidence)
            .bind(&now)
            .bind(article_id)
            .execute(&self.pool)
            .await?;
        }
        _ => {
            // "uncertain" — store AI fields but leave status as 'retrieved'
            sqlx::query(
                "UPDATE article SET ai_rationale = ?, ai_confidence = ?, updated_at = ? WHERE id = ?",
            )
            .bind(rationale)
            .bind(confidence)
            .bind(&now)
            .bind(article_id)
            .execute(&self.pool)
            .await?;
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Update `clear_exclusion` to also wipe AI fields (~line 854)**

Find:
```rust
pub async fn clear_exclusion(&self, article_id: i64) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE article SET status = 'retrieved', exclusion_criterion = NULL, exclusion_rationale = NULL, screened_at = NULL, updated_at = ? WHERE id = ?",
    )
```

Replace with:
```rust
pub async fn clear_exclusion(&self, article_id: i64) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE article SET status = 'retrieved', exclusion_criterion = NULL, \
         exclusion_rationale = NULL, screened_at = NULL, \
         ai_rationale = NULL, ai_confidence = NULL, updated_at = ? WHERE id = ?",
    )
```

- [ ] **Step 5: Build**

```bash
cargo build -p reeves 2>&1 | grep "^error"
```

Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): tab pagination, count, apply_ai_outcome, clear AI fields on revert"
```

---

### Task 4: suggest.rs — screen_article Function

**Files:**
- Modify: `src-tauri/src/suggest.rs`

- [ ] **Step 1: Add the result type at the top of suggest.rs (after existing types)**

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScreeningDecision {
    pub decision: String,         // "include" | "exclude" | "uncertain"
    pub criterion_number: Option<i64>,
    pub rationale: String,
    pub confidence: f64,
}
```

- [ ] **Step 2: Add `screen_article` function at the end of suggest.rs**

```rust
/// Screen a single article against inclusion/exclusion criteria.
/// Returns a ScreeningDecision with decision, optional criterion, rationale, and confidence.
pub async fn screen_article(
    base_url: &str,
    model: &str,
    title: &str,
    abstract_text: Option<&str>,
    inclusion_criteria: &[String],
    exclusion_criteria_labeled: &[(i64, String)], // (criterion_number, label)
    api_key: Option<&str>,
) -> Result<ScreeningDecision> {
    let inc_block = if inclusion_criteria.is_empty() {
        "(none specified)".to_string()
    } else {
        inclusion_criteria
            .iter()
            .map(|c| format!("- {c}"))
            .collect::<Vec<_>>()
            .join("\n")
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
         Inclusion criteria (article must meet ALL to be included):\n{inc_block}\n\n\
         Exclusion criteria (article is excluded if it meets ANY):\n{exc_block}\n\n\
         Respond ONLY with a JSON object with exactly four keys:\n\
         - \"decision\": \"include\", \"exclude\", or \"uncertain\"\n\
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
        .ok_or_else(|| anyhow!("could not parse screening decision from model output: {}", result.content))
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
    let decision = v.get("decision")?.as_str()?.to_string();
    if !["include", "exclude", "uncertain"].contains(&decision.as_str()) {
        return None;
    }
    let criterion_number = v.get("criterion_number").and_then(|x| x.as_i64());
    let rationale = v.get("rationale")?.as_str()?.to_string();
    let confidence = v.get("confidence")?.as_f64()?;
    Some(ScreeningDecision { decision, criterion_number, rationale, confidence })
}
```

- [ ] **Step 3: Build**

```bash
cargo build -p reeves 2>&1 | grep "^error"
```

Expected: no output.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/suggest.rs
git commit -m "feat(suggest): add screen_article for AI title/abstract screening"
```

---

### Task 5: New Rust Commands

**Files:**
- Modify: `src-tauri/src/lib.rs`

Three new commands: `run_ai_screening`, `list_screening_tab`, `count_screening_tab`.

- [ ] **Step 1: Add arg structs near the existing screening arg structs (~line 810)**

```rust
#[derive(Debug, Deserialize)]
struct ScreeningTabArgs {
    tab: String,
    offset: i64,
    limit: i64,
}

#[derive(Debug, Serialize)]
struct AiScreeningProgress {
    done: usize,
    total: usize,
    article_title: String,
    decision: String, // "include" | "exclude" | "uncertain" | "error"
}

#[derive(Debug, Serialize)]
struct AiScreeningResult {
    processed: usize,
    included: usize,
    excluded: usize,
    uncertain: usize,
}

#[derive(Debug, Deserialize)]
struct RunAiScreeningArgs {
    inclusion_criteria: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_vec")]
    exclusion_criteria: Vec<String>,
}
```

- [ ] **Step 2: Add `list_screening_tab` command after `list_screening_batch`**

```rust
#[tauri::command]
async fn list_screening_tab(
    args: ScreeningTabArgs,
    state: State<'_, AppState>,
) -> Result<Vec<ArticleRow>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_articles_for_tab(&args.tab, args.offset, args.limit)
        .await
        .map_err(err)
}

#[tauri::command]
async fn count_screening_tab(
    tab: String,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.count_articles_for_tab(&tab).await.map_err(err)
}
```

- [ ] **Step 3: Add `run_ai_screening` command**

```rust
#[tauri::command]
async fn run_ai_screening(
    args: RunAiScreeningArgs,
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
) -> Result<AiScreeningResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;

    let base_url = settings
        .get("lmstudio.base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:1234/v1")
        .to_string();
    let api_key = settings
        .get("lmstudio.api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let model = settings
        .get("lmstudio.chat_model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if model.is_empty() {
        return Err("Pick a chat model in Settings first.".into());
    }
    let threshold: f64 = settings
        .get("screening.confidence_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.90);

    // Load all pending articles (status = 'retrieved')
    let pending = db.list_articles_for_tab("pending", 0, i64::MAX).await.map_err(err)?;
    let total = pending.len();
    let mut included = 0usize;
    let mut excluded = 0usize;
    let mut uncertain = 0usize;

    // Build labeled exclusion criteria list from the DB
    let criteria = db.list_criteria().await.map_err(err)?;
    let exc_labeled: Vec<(i64, String)> = criteria
        .iter()
        .filter(|c| c.number > 1)
        .map(|c| (c.number as i64, c.label.clone()))
        .collect();

    for (i, article) in pending.iter().enumerate() {
        let _ = app.emit(
            "screen:progress",
            AiScreeningProgress {
                done: i,
                total,
                article_title: article.title.clone(),
                decision: "screening".into(),
            },
        );

        let result = suggest::screen_article(
            &base_url,
            &model,
            &article.title,
            article.abstract_text.as_deref(),
            &args.inclusion_criteria,
            &exc_labeled,
            api_key.as_deref(),
        )
        .await;

        match result {
            Ok(decision) => {
                let applies = decision.confidence >= threshold && decision.decision != "uncertain";
                if applies {
                    db.apply_ai_outcome(
                        article.id,
                        &decision.decision,
                        decision.criterion_number,
                        &decision.rationale,
                        decision.confidence,
                    )
                    .await
                    .map_err(err)?;
                    match decision.decision.as_str() {
                        "include" => included += 1,
                        "exclude" => excluded += 1,
                        _ => uncertain += 1,
                    }
                } else {
                    // Below threshold or explicit uncertain — store but don't apply
                    db.apply_ai_outcome(
                        article.id,
                        "uncertain",
                        None,
                        &decision.rationale,
                        decision.confidence,
                    )
                    .await
                    .map_err(err)?;
                    uncertain += 1;
                }
                let _ = app.emit(
                    "screen:progress",
                    AiScreeningProgress {
                        done: i + 1,
                        total,
                        article_title: article.title.clone(),
                        decision: if applies { decision.decision.clone() } else { "uncertain".into() },
                    },
                );
            }
            Err(e) => {
                eprintln!("AI screening error for article {}: {e}", article.id);
                let _ = app.emit(
                    "screen:progress",
                    AiScreeningProgress {
                        done: i + 1,
                        total,
                        article_title: article.title.clone(),
                        decision: "error".into(),
                    },
                );
            }
        }
    }

    db.record_telemetry(
        "screen.ai_run",
        &serde_json::json!({
            "total": total,
            "included": included,
            "excluded": excluded,
            "uncertain": uncertain,
            "model": model,
        }),
    )
    .await
    .ok();

    Ok(AiScreeningResult { processed: total, included, excluded, uncertain })
}
```

- [ ] **Step 4: Register the three new commands in the `.invoke_handler` list (~line 2050)**

Find the block that starts with `.invoke_handler(tauri::generate_handler![` and add inside it:

```rust
list_screening_tab,
count_screening_tab,
run_ai_screening,
```

- [ ] **Step 5: Build**

```bash
cargo build -p reeves 2>&1 | grep "^error"
```

Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(commands): list_screening_tab, count_screening_tab, run_ai_screening"
```

---

### Task 6: Settings — Confidence Threshold

**Files:**
- Modify: `src/lib/useAppState.ts`
- Modify: `src/views/Settings.tsx`

- [ ] **Step 1: Add the key to `AppSettings` in `useAppState.ts`**

In the `AppSettings` interface, add after `"telemetry.log_reasoning"`:

```ts
/** AI screening confidence threshold 0–1; decisions below this are flagged for manual review */
"screening.confidence_threshold": number;
```

In the `DEFAULTS` object add:

```ts
"screening.confidence_threshold": 0.9,
```

- [ ] **Step 2: Add the setting field in `Settings.tsx`**

Find the AI models section (look for `"lmstudio.chat_model"` usage around line 205). After the embed model row, add a new row for the threshold. Find the pattern and insert:

```tsx
<SettingRow
  icon={<Filter size={14} strokeWidth={1.75} />}
  label="Screening confidence threshold"
  description="AI decisions with confidence below this value are flagged for manual review instead of auto-applied."
>
  <input
    type="number"
    min={0.5}
    max={1}
    step={0.05}
    value={settings["screening.confidence_threshold"]}
    onChange={(e) => {
      const v = parseFloat(e.target.value);
      if (!isNaN(v) && v >= 0.5 && v <= 1) {
        onUpdate("screening.confidence_threshold", v);
      }
    }}
    className="w-20 px-2.5 py-1.5 text-[13px] font-mono rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white focus:outline-none focus:ring-1 focus:ring-[var(--color-paper-400)]"
  />
</SettingRow>
```

You will need to add `Filter` to the lucide-react import at the top of `Settings.tsx`.

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio && npm run build 2>&1 | grep -E "error TS|Error"
```

Expected: no TypeScript errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/useAppState.ts src/views/Settings.tsx
git commit -m "feat(settings): add AI screening confidence threshold"
```

---

### Task 7: Frontend API Bridge

**Files:**
- Modify: `src/lib/api.ts`

- [ ] **Step 1: Update `ArticleRow` interface (around line 122)**

Add two fields:

```ts
export interface ArticleRow {
  id: number;
  title: string;
  authors: string | null;
  year: number | null;
  journal: string | null;
  abstract_text: string | null;
  doi: string | null;
  url: string | null;
  source_databases: string | null;
  status: string;
  dedup_group_id: number | null;
  exclusion_criterion: number | null;
  created_at: string;
  /** Set only when the user explicitly screened (included/excluded) this article. */
  screened_at: string | null;
  /** Rationale recorded by the AI screener. Null if human-screened. */
  ai_rationale: string | null;
  /** Confidence score 0–1 from AI screener. Null if human-screened. */
  ai_confidence: number | null;
}
```

- [ ] **Step 2: Add new types after `ExclusionCriterionRow`**

```ts
export type ScreeningTab = "pending" | "included" | "excluded";

export interface AiScreeningProgress {
  done: number;
  total: number;
  article_title: string;
  decision: "include" | "exclude" | "uncertain" | "screening" | "error";
}

export interface AiScreeningResult {
  processed: number;
  included: number;
  excluded: number;
  uncertain: number;
}
```

- [ ] **Step 3: Add the three new API functions (after `listScreeningBatch`)**

```ts
listScreeningTab(tab: ScreeningTab, offset: number, limit: number) {
  return invoke<ArticleRow[]>("list_screening_tab", { args: { tab, offset, limit } });
},
countScreeningTab(tab: ScreeningTab) {
  return invoke<number>("count_screening_tab", { tab });
},
runAiScreening(inclusionCriteria: string[], exclusionCriteria: string[]) {
  return invoke<AiScreeningResult>("run_ai_screening", {
    args: { inclusion_criteria: inclusionCriteria, exclusion_criteria: exclusionCriteria },
  });
},
```

- [ ] **Step 4: Verify TypeScript compiles**

```bash
npm run build 2>&1 | grep -E "error TS|Error"
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): ArticleRow AI fields, listScreeningTab, countScreeningTab, runAiScreening"
```

---

### Task 8: AiScreeningModal Component

**Files:**
- Create: `src/components/AiScreeningModal.tsx`

This modal has three steps: `"checklist"` → `"extra"` → `"running"`.

- [ ] **Step 1: Create `src/components/AiScreeningModal.tsx`**

```tsx
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { motion, AnimatePresence } from "framer-motion";
import { Bot, Check, ChevronRight, Loader2, Plus, Sparkles, X } from "lucide-react";
import { api, type AiScreeningProgress, type AiScreeningResult, type ExclusionCriterionRow, type ProjectRow } from "../lib/api";
import { Button } from "./ui/Button";
import { Card } from "./ui/Card";
import { cn } from "../lib/utils";

interface Props {
  open: boolean;
  project: ProjectRow;
  criteria: ExclusionCriterionRow[]; // EC list from DB (number > 1)
  onComplete: (result: AiScreeningResult) => void;
  onDismiss: () => void;
}

type Step = "checklist" | "extra" | "running" | "done";

// Standard PRISMA criteria always shown in the checklist.
// These mirror the seeded exclusion criteria in 0002_arranging.sql (EC2–EC9).
const STANDARD_CRITERIA = [
  { number: 2, label: "Outside search period" },
  { number: 3, label: "Below source quality threshold" },
  { number: 4, label: "Wrong source type (e.g. conference proceedings, book chapters)" },
  { number: 5, label: "Non-English publication" },
  { number: 6, label: "Predatory journal" },
  { number: 7, label: "Full text not accessible" },
  { number: 8, label: "Not directly relevant to domain" },
  { number: 9, label: "Retracted article" },
];

export function AiScreeningModal({ open, project, criteria, onComplete, onDismiss }: Props) {
  const [step, setStep] = useState<Step>("checklist");

  // Step 1 — checklist
  const [selectedEc, setSelectedEc] = useState<Set<number>>(new Set());
  const [inclusions, setInclusions] = useState<string[]>([]);

  // Step 2 — extra criteria
  const [wantsExtra, setWantsExtra] = useState<boolean | null>(null);
  const [extraInput, setExtraInput] = useState("");

  // Step 3 — progress
  const [progress, setProgress] = useState<AiScreeningProgress | null>(null);
  const [result, setResult] = useState<AiScreeningResult | null>(null);
  const [runError, setRunError] = useState<string | null>(null);

  // Pre-populate checklist from project Stage 1 criteria
  useEffect(() => {
    if (!open) return;
    setStep("checklist");
    setSelectedEc(new Set(STANDARD_CRITERIA.map((c) => c.number)));
    const inc: string[] = project.search_inclusion_criteria
      ? (JSON.parse(project.search_inclusion_criteria) as string[])
      : [];
    setInclusions(inc);
    setWantsExtra(null);
    setExtraInput("");
    setProgress(null);
    setResult(null);
    setRunError(null);
  }, [open]);

  // Escape key
  useEffect(() => {
    if (!open || step === "running") return;
    const handler = (e: KeyboardEvent) => { if (e.key === "Escape") onDismiss(); };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, step, onDismiss]);

  const toggleEc = (n: number) => {
    setSelectedEc((prev) => {
      const next = new Set(prev);
      if (next.has(n)) next.delete(n);
      else next.add(n);
      return next;
    });
  };

  const handleExtraDecision = (yes: boolean) => {
    setWantsExtra(yes);
    if (!yes) startRun();
    else setStep("extra");
  };

  const handleExtraSubmit = () => {
    startRun(extraInput.trim() || undefined);
  };

  const startRun = async (extraCriterion?: string) => {
    setStep("running");
    setRunError(null);

    const exclusionCriteria = STANDARD_CRITERIA
      .filter((c) => selectedEc.has(c.number))
      .map((c) => `EC${c.number}: ${c.label}`);
    if (extraCriterion) exclusionCriteria.push(extraCriterion);

    const unlisten = await listen<AiScreeningProgress>("screen:progress", (e) => {
      setProgress(e.payload);
    });

    try {
      const res = await api.runAiScreening(inclusions, exclusionCriteria);
      setResult(res);
      setStep("done");
    } catch (e: any) {
      const msg: string = e?.message ?? String(e);
      setRunError(
        msg.includes("chat model")
          ? "Pick a chat model in Settings first."
          : msg.includes("Connection refused") || msg.includes("error sending request")
          ? "Could not reach LM Studio — check Settings."
          : msg,
      );
    } finally {
      unlisten();
    }
  };

  return (
    <AnimatePresence>
      {open && (
        <>
          <motion.div
            key="backdrop"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="fixed inset-0 bg-[var(--color-ink-900)]/30 backdrop-blur-sm z-40"
            onClick={step !== "running" ? onDismiss : undefined}
          />
          <motion.div
            key="modal"
            initial={{ opacity: 0, scale: 0.97, y: 8 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.97, y: 8 }}
            transition={{ duration: 0.2, ease: [0.2, 0.7, 0.2, 1] }}
            className="fixed inset-0 z-50 flex items-center justify-center p-6 pointer-events-none"
          >
            <Card
              role="dialog"
              aria-modal="true"
              className="pointer-events-auto w-full max-w-xl p-6 shadow-[var(--shadow-paper-md)]"
            >
              {/* Header */}
              <div className="flex items-start justify-between mb-5">
                <div className="flex items-center gap-2">
                  <Sparkles size={15} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                  <span className="eyebrow">Screen with AI</span>
                </div>
                {step !== "running" && (
                  <button
                    onClick={onDismiss}
                    className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
                  >
                    <X size={14} strokeWidth={1.75} />
                  </button>
                )}
              </div>

              {/* Step: checklist */}
              {step === "checklist" && (
                <div>
                  <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed mb-4">
                    Select the exclusion criteria the AI should apply. Pre-selected from your
                    project configuration.
                  </p>

                  {inclusions.length > 0 && (
                    <div className="mb-4">
                      <p className="text-[11px] font-semibold uppercase tracking-wide text-[var(--color-paper-500)] mb-2">
                        Inclusion criteria (from Stage 1)
                      </p>
                      <ul className="space-y-1">
                        {inclusions.map((inc, i) => (
                          <li
                            key={i}
                            className="flex items-center gap-2 text-[12px] text-[var(--color-ink-700)]"
                          >
                            <Check size={11} strokeWidth={2.5} className="text-[var(--color-ok-500)] shrink-0" />
                            {inc}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}

                  <p className="text-[11px] font-semibold uppercase tracking-wide text-[var(--color-paper-500)] mb-2">
                    Exclusion criteria
                  </p>
                  <ul className="space-y-1.5 max-h-64 overflow-y-auto pr-1">
                    {STANDARD_CRITERIA.map((c) => (
                      <li key={c.number}>
                        <button
                          type="button"
                          onClick={() => toggleEc(c.number)}
                          className={cn(
                            "w-full text-left flex items-center gap-3 px-3 py-2 rounded-[var(--radius-sm)] border text-[12px] transition-paper",
                            selectedEc.has(c.number)
                              ? "bg-white border-[var(--color-ink-700)] text-[var(--color-ink-800)]"
                              : "bg-[var(--color-paper-50)] border-[var(--color-paper-300)] text-[var(--color-paper-500)]",
                          )}
                        >
                          <div
                            className={cn(
                              "w-4 h-4 rounded-[3px] border-2 flex items-center justify-center shrink-0 transition-paper",
                              selectedEc.has(c.number)
                                ? "bg-[var(--color-ink-700)] border-[var(--color-ink-700)]"
                                : "border-[var(--color-paper-400)]",
                            )}
                          >
                            {selectedEc.has(c.number) && (
                              <svg width="9" height="9" viewBox="0 0 9 9">
                                <path d="M1 4.5L3.5 7L8 1.5" stroke="white" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round" />
                              </svg>
                            )}
                          </div>
                          <span>
                            <span className="font-mono text-[10px] text-[var(--color-paper-400)] mr-1.5">EC{c.number}</span>
                            {c.label}
                          </span>
                        </button>
                      </li>
                    ))}
                  </ul>

                  <div className="mt-5 flex items-center justify-between">
                    <span className="text-[12px] text-[var(--color-paper-500)]">
                      {selectedEc.size} criteria selected
                    </span>
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={() => setStep("extra")}
                    >
                      Continue
                      <ChevronRight size={13} strokeWidth={2} />
                    </Button>
                  </div>
                </div>
              )}

              {/* Step: extra criteria */}
              {step === "extra" && wantsExtra === null && (
                <div>
                  <p className="text-[14px] text-[var(--color-ink-800)] font-medium mb-2">
                    Any additional criteria?
                  </p>
                  <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed mb-5">
                    Add a domain-specific criterion the standard list doesn't cover.
                  </p>
                  <div className="flex gap-3">
                    <Button variant="primary" size="sm" onClick={() => setWantsExtra(true)}>
                      <Plus size={13} strokeWidth={2} />
                      Yes, add one
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => startRun()}>
                      No, begin screening
                      <Bot size={13} strokeWidth={1.75} />
                    </Button>
                  </div>
                </div>
              )}

              {step === "extra" && wantsExtra === true && (
                <div>
                  <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed mb-3">
                    Describe the additional criterion in plain language.
                  </p>
                  <textarea
                    autoFocus
                    value={extraInput}
                    onChange={(e) => setExtraInput(e.target.value)}
                    rows={3}
                    placeholder="e.g. Must study adult populations only (exclude studies on children under 18)"
                    className="w-full px-3 py-2 text-[13px] rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] bg-white placeholder:text-[var(--color-paper-400)] focus:outline-none focus:ring-1 focus:ring-[var(--color-paper-400)] resize-none"
                  />
                  <div className="mt-4 flex gap-3">
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={handleExtraSubmit}
                      disabled={!extraInput.trim()}
                    >
                      Begin screening
                      <Bot size={13} strokeWidth={1.75} />
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => startRun()}>
                      Skip and begin
                    </Button>
                  </div>
                </div>
              )}

              {/* Step: running */}
              {step === "running" && (
                <div>
                  {runError ? (
                    <div>
                      <p className="text-[13px] text-[var(--color-warn-700)] mb-4">{runError}</p>
                      <Button variant="ghost" size="sm" onClick={onDismiss}>Dismiss</Button>
                    </div>
                  ) : (
                    <div>
                      <div className="flex items-center gap-2 mb-4 text-[13px] text-[var(--color-paper-600)]">
                        <Loader2 size={14} className="animate-spin shrink-0" />
                        <span className="truncate">
                          {progress
                            ? `Screening: ${progress.article_title}`
                            : "Starting…"}
                        </span>
                      </div>

                      {/* Progress bar */}
                      <div className="w-full h-2 rounded-full bg-[var(--color-paper-200)] overflow-hidden mb-2">
                        <motion.div
                          className="h-full rounded-full bg-[var(--color-accent-500)]"
                          initial={{ width: 0 }}
                          animate={{
                            width: progress && progress.total > 0
                              ? `${(progress.done / progress.total) * 100}%`
                              : "0%",
                          }}
                          transition={{ ease: "easeOut", duration: 0.3 }}
                        />
                      </div>

                      <div className="flex items-center justify-between text-[11px] text-[var(--color-paper-500)] tabular-nums">
                        <span>
                          {progress ? `${progress.done} / ${progress.total}` : "—"}
                        </span>
                        {progress && (
                          <span className={cn(
                            "font-medium",
                            progress.decision === "include" && "text-[var(--color-ok-600)]",
                            progress.decision === "exclude" && "text-[var(--color-warn-600)]",
                            progress.decision === "uncertain" && "text-[var(--color-paper-500)]",
                            progress.decision === "error" && "text-[var(--color-error-500)]",
                          )}>
                            {progress.decision === "include" && "Included"}
                            {progress.decision === "exclude" && "Excluded"}
                            {progress.decision === "uncertain" && "Flagged for review"}
                            {progress.decision === "screening" && "Screening…"}
                            {progress.decision === "error" && "Error — skipped"}
                          </span>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              )}

              {/* Step: done */}
              {step === "done" && result && (
                <div>
                  <p className="text-[14px] font-medium text-[var(--color-ink-800)] mb-4">
                    Screening complete
                  </p>
                  <div className="grid grid-cols-3 gap-3 mb-5">
                    <StatPill label="Included" value={result.included} color="ok" />
                    <StatPill label="Excluded" value={result.excluded} color="warn" />
                    <StatPill label="Needs review" value={result.uncertain} color="neutral" />
                  </div>
                  <p className="text-[12px] text-[var(--color-paper-500)] leading-relaxed mb-4">
                    Articles flagged for review remain in Pending Screening with an AI rationale.
                    All decisions can be reverted from the article card.
                  </p>
                  <Button variant="primary" size="sm" onClick={() => onComplete(result)}>
                    View results
                  </Button>
                </div>
              )}
            </Card>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}

function StatPill({ label, value, color }: { label: string; value: number; color: "ok" | "warn" | "neutral" }) {
  const cls = {
    ok: "bg-[var(--color-ok-500)]/10 text-[var(--color-ok-600)]",
    warn: "bg-[var(--color-warn-500)]/10 text-[var(--color-warn-600)]",
    neutral: "bg-[var(--color-paper-100)] text-[var(--color-paper-600)]",
  }[color];
  return (
    <div className={cn("rounded-[var(--radius-sm)] px-3 py-2 text-center", cls)}>
      <div className="text-[22px] font-serif tracking-tight">{value}</div>
      <div className="text-[11px] font-medium mt-0.5">{label}</div>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
npm run build 2>&1 | grep -E "error TS|Error"
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/AiScreeningModal.tsx
git commit -m "feat(ui): AiScreeningModal with checklist, extra criteria, and progress bar"
```

---

### Task 9: Refactor ScreeningPanel to Three Tabs + Integrate Modal

**Files:**
- Modify: `src/views/arranging/ScreeningPanel.tsx`

Replace the current single-list panel with a three-tab layout. The `ScreeningCard` component and all its helpers (`JournalBadge`, journal quality logic) stay intact — only the outer shell changes.

- [ ] **Step 1: Replace the imports and add new ones at the top of `ScreeningPanel.tsx`**

```tsx
import { useEffect, useMemo, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  Bot,
  Check,
  ChevronDown,
  ChevronUp,
  Filter,
  Loader2,
  RotateCcw,
  Sparkles,
} from "lucide-react";
import {
  api,
  type AiScreeningResult,
  type ArticleRow,
  type ExclusionCriterionRow,
  type JournalRow,
  type ProjectRow,
  type ScreeningTab,
} from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { AiScreeningModal } from "../../components/AiScreeningModal";
import { cn } from "../../lib/utils";
```

- [ ] **Step 2: Replace the `ScreeningPanel` component function entirely**

Replace everything from `export function ScreeningPanel` through the closing `}` of the component (but keep `ScreeningCard`, `JournalBadge`, and helper functions below unchanged):

```tsx
const BATCH_SIZE = 20;
const TABS: { id: ScreeningTab; label: string }[] = [
  { id: "pending", label: "Pending Screening" },
  { id: "included", label: "Included" },
  { id: "excluded", label: "Excluded" },
];

export function ScreeningPanel({ project, refreshKey, onChanged }: Props) {
  const [activeTab, setActiveTab] = useState<ScreeningTab>("pending");
  const [articles, setArticles] = useState<ArticleRow[]>([]);
  const [total, setTotal] = useState(0);
  const [criteria, setCriteria] = useState<ExclusionCriterionRow[]>([]);
  const [journals, setJournals] = useState<JournalRow[]>([]);
  const [offset, setOffset] = useState(0);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [loading, setLoading] = useState(true);
  const [aiModalOpen, setAiModalOpen] = useState(false);

  const jq = parseJournalQuality(project.journal_quality);

  const reload = async (tab = activeTab, off = offset) => {
    setLoading(true);
    const [arts, cnt, crits, jList] = await Promise.all([
      api.listScreeningTab(tab, off, BATCH_SIZE),
      api.countScreeningTab(tab),
      api.listCriteria(),
      api.listJournals(),
    ]);
    setArticles(arts);
    setTotal(cnt);
    setCriteria(crits.filter((c) => c.number > 1));
    setJournals(jList);
    setLoading(false);
  };

  useEffect(() => {
    setOffset(0);
    reload(activeTab, 0);
  }, [refreshKey, activeTab]);

  const journalMap = useMemo(() => {
    const m = new Map<string, JournalRow>();
    for (const j of journals) m.set(normalizeJournal(j.name), j);
    return m;
  }, [journals]);

  const lookupJournal = (articleJournal: string | null | undefined): JournalRow | null => {
    if (!articleJournal) return null;
    return journalMap.get(normalizeJournal(articleJournal)) ?? null;
  };

  const onDecision = async () => {
    onChanged();
    await reload();
  };

  const handleAiComplete = (result: AiScreeningResult) => {
    setAiModalOpen(false);
    onChanged();
    reload("pending", 0);
  };

  const totalPages = Math.max(1, Math.ceil(total / BATCH_SIZE));
  const currentPage = Math.floor(offset / BATCH_SIZE) + 1;

  return (
    <div className="pl-6 pr-4 xl:pl-12 xl:pr-8">
      {/* Header row */}
      <div className="flex items-end justify-between mb-5">
        <div>
          <div className="eyebrow text-[var(--color-paper-500)]">Title & abstract screening</div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">
            Decide one at a time
          </h2>
        </div>
        {activeTab === "pending" && (
          <button
            type="button"
            onClick={() => setAiModalOpen(true)}
            className="no-drag inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-accent-600)] border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)] transition-paper"
          >
            <Sparkles size={12} strokeWidth={2} />
            Screen with AI
          </button>
        )}
      </div>

      {/* Tabs */}
      <div className="flex items-center gap-1 mb-5 border-b border-[var(--color-paper-200)]">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => { setActiveTab(t.id); setOffset(0); }}
            className={cn(
              "px-4 py-2 text-[13px] font-medium transition-paper -mb-px border-b-2",
              activeTab === t.id
                ? "border-[var(--color-ink-800)] text-[var(--color-ink-800)]"
                : "border-transparent text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)]",
            )}
          >
            {t.label}
            {!loading && (
              <span className={cn(
                "ml-1.5 text-[11px] tabular-nums",
                activeTab === t.id ? "text-[var(--color-paper-500)]" : "text-[var(--color-paper-400)]",
              )}>
                {t.id === activeTab ? total : ""}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Pagination */}
      <div className="flex items-center justify-between mb-4">
        <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed max-w-xl">
          {activeTab === "pending"
            ? "Keep each article or apply an exclusion criterion. AI-flagged articles show the AI rationale."
            : activeTab === "included"
            ? "Articles kept after screening. Click Revert to return an article to Pending."
            : "Articles excluded from the corpus. Click Revert to return an article to Pending."}
        </p>
        <div className="flex items-center gap-1 shrink-0">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setOffset(Math.max(0, offset - BATCH_SIZE))}
            disabled={offset === 0}
          >
            <ArrowLeft size={13} strokeWidth={2} />
            Prev
          </Button>
          <span className="px-3 text-[12px] font-semibold tabular-nums text-[var(--color-ink-700)]">
            {currentPage} / {totalPages}
          </span>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setOffset(offset + BATCH_SIZE)}
            disabled={offset + BATCH_SIZE >= total}
          >
            Next
            <ArrowRight size={13} strokeWidth={2} />
          </Button>
        </div>
      </div>

      {loading && (
        <div className="flex items-center gap-2 text-[13px] text-[var(--color-paper-500)]">
          <Loader2 size={14} className="animate-spin" /> Loading…
        </div>
      )}

      {!loading && articles.length === 0 && (
        <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-12 text-center">
          <Filter className="mx-auto mb-3 text-[var(--color-paper-400)]" size={24} strokeWidth={1.5} />
          <p className="text-[14px] text-[var(--color-paper-700)] font-medium mb-1">
            {activeTab === "pending" ? "Nothing to screen" : activeTab === "included" ? "No articles included yet" : "No articles excluded yet"}
          </p>
          <p className="text-[13px] text-[var(--color-paper-500)] max-w-md mx-auto leading-relaxed">
            {activeTab === "pending"
              ? "Import records in Stage 1 or all articles have already been screened."
              : activeTab === "included"
              ? "Include articles from the Pending tab to see them here."
              : "Exclude articles from the Pending tab to see them here."}
          </p>
        </div>
      )}

      <div className="space-y-3">
        {articles.map((a) => (
          <ScreeningCard
            key={a.id}
            article={a}
            criteria={criteria}
            journalMatch={lookupJournal(a.journal)}
            jq={jq}
            expanded={expanded.has(a.id)}
            onToggle={() => {
              setExpanded((s) => {
                const n = new Set(s);
                if (n.has(a.id)) n.delete(a.id);
                else n.add(a.id);
                return n;
              });
            }}
            onDecision={onDecision}
          />
        ))}
      </div>

      {!loading && articles.some((a) => lookupJournal(a.journal)?.impact_factor != null) && (
        <p className="mt-6 text-[11px] text-[var(--color-paper-400)] leading-relaxed max-w-2xl">
          * <strong className="font-medium text-[var(--color-paper-500)]">IF</strong> — Impact
          factor sourced from your journal scope list. Use as a screening guide only.
        </p>
      )}

      <AiScreeningModal
        open={aiModalOpen}
        project={project}
        criteria={criteria}
        onComplete={handleAiComplete}
        onDismiss={() => setAiModalOpen(false)}
      />
    </div>
  );
}
```

- [ ] **Step 3: Update `ScreeningCard` to show AI rationale and uncertainty label**

In the existing `ScreeningCard` component, find the section after the quality warning block and add the AI rationale display. Insert before the abstract toggle button:

```tsx
{/* AI rationale — shown for AI-screened articles or AI-uncertain articles */}
{a.ai_rationale && (
  <div className="mt-2 flex items-start gap-1.5 text-[11px]">
    <Bot size={11} strokeWidth={1.75} className="shrink-0 mt-0.5 text-[var(--color-accent-500)]" />
    <span className={cn(
      "leading-snug",
      a.ai_confidence !== null && a.status === "retrieved"
        ? "text-[var(--color-paper-500)] italic"
        : "text-[var(--color-paper-500)]",
    )}>
      {a.ai_confidence !== null && a.status === "retrieved" && (
        <span className="font-medium text-[var(--color-warn-600)] not-italic mr-1">
          AI couldn't decide —
        </span>
      )}
      {a.ai_rationale}
      {a.ai_confidence !== null && (
        <span className="ml-1 text-[var(--color-paper-400)]">
          ({Math.round(a.ai_confidence * 100)}% confidence)
        </span>
      )}
    </span>
  </div>
)}
```

Note: this code goes inside `ScreeningCard`, inside the `<div className="flex-1 min-w-0">` div, after the quality warning block and before the abstract toggle `<button>`.

- [ ] **Step 4: Verify TypeScript compiles**

```bash
npm run build 2>&1 | grep -E "error TS|Error"
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/views/arranging/ScreeningPanel.tsx
git commit -m "feat(ui): three-tab ScreeningPanel with AI screening integration"
```

---

## Final Verification

- [ ] Build the full app and open it

```bash
npm run tauri dev
```

- [ ] Open a project, navigate to Stage 2 → Purification → Screening
- [ ] Confirm three tabs appear: Pending Screening, Included, Excluded
- [ ] Confirm pagination works independently per tab
- [ ] Click "Screen with AI" — checklist modal appears with pre-selected EC2–EC9
- [ ] Step through: select criteria → Yes/No for extra → "Begin screening"
- [ ] Progress bar advances article by article
- [ ] After completion: summary modal shows counts, "View results" closes modal
- [ ] AI-uncertain articles appear in Pending with "AI couldn't decide" label and rationale
- [ ] Included/Excluded articles show AI rationale on their cards
- [ ] Reverting an AI-decided article sends it back to Pending (clean — no AI label)
- [ ] Settings panel shows the confidence threshold field

- [ ] **Final commit**

```bash
git add -A
git commit -m "feat(screening): AI-assisted screening with three-tab panel and confidence threshold"
```
