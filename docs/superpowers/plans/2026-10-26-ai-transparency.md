# AI Transparency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every AI screening decision, coding suggestion, merge and consolidation in Reeves leaves a complete, attributable record. The researcher can see what was applied automatically, what they opened and what they overrode. The app and README stop claiming things the app does not do.

**Architecture:** A new `ai_call` table stores provenance for every model call: the requested model, the model actually used, a prompt fingerprint, the temperature and the seed. Screening, coding, merge and consolidation rows point at it by `ai_call_id`. The `ai_screening_proposal` table is rebuilt into the full screening log: stage, mode, nullable confidence, auto-applied flag and override fields, plus an `ai_screening_open` event table. Review status is derived from those rows, never stored. All decision logic that the tests pin down is pulled out into pure functions in `src-tauri/src/transparency.rs` and tested there. Commands in `lib.rs` stay thin.

**Tech Stack:** Tauri v2, Rust (sqlx/SQLite, tokio), React 19 + TypeScript, Tailwind.

**Spec:** `docs/superpowers/specs/2026-10-26-ai-transparency-requirements.md`

## Global Constraints

- The word "adjudicator" must not appear in any user-visible string: UI, report, progress labels, Settings or the README. Internal identifiers (`ai_adjudication` table, `model-ai-adjudicated` source, `undo_adjudication` command) keep their names.
- UI and report say "opened" only. Never say "read" or "reviewed" about AI decisions.
- The four review statuses use exactly these labels: **Applied automatically**, **Opened, left unchanged**, **Overridden**, **Pending**. A decision that was applied automatically and never opened also shows **Never opened** in the opens column.
- Auto-apply threshold stays `screening.confidence_threshold` (default `0.90`), compared with `>=`.
- The coding-confidence label reads: "Model's own estimate, not a measured probability."
- The seed setting `ai.seed` is off by default (`null`). No text may claim runs are exactly repeatable. The report wording is "Recorded to help others re-run the analysis; model outputs may still vary between runs."
- Agreed-codes setting key: `coding.agreed_codes_mode`, values `"auto"` | `"suggest"`. Existing projects get `"auto"`; new projects default to `"suggest"`.
- Suggestion labels: **Agreed by both agents**, **One agent only**.
- Reminder buttons: **Open them now**, **Continue anyway**. Reminders never disable or block navigation or export.
- Model-match warning text: "This model is also a coding agent. A model may favour its own wording when merging."
- New editable prompt settings: `prompts.merging`, `prompts.consolidating`, `prompts.synthesis`. A blank value means use the default. With blank settings, the default plus the fixed output format must equal today's system prompt byte for byte. The only change is the word swap in Task 8.
- Tested platforms: LM Studio on macOS. Ollama "should work but is untested".

## Review Focus

1. **Re-running AI screening on still-pending articles** writes a second log row for the same article. Badges, reminders and status must use only the latest row per (article, stage). Older rows stay in the report. Test: `latest_per_article_stage_wins` (Task 2).
2. **A pending AI decision on an article that leaves its stage by another route** (quality-exclusion rule, purification, revert) must not keep counting in the reminder. Test: `attention_counts_ignore_articles_no_longer_undecided` (Task 2).
3. **Model call errors and missing PDFs** must leave a log row, but with no review status. They are not AI decisions and must not inflate reminder counts. Test: `errors_and_skips_have_no_status` (Task 2).
4. **Cancelled AI coding run in suggestion mode**: `revert_ai_coding` must also delete `agreement='both'` proposals from that run, otherwise half a run lingers as suggestions. Test: `revert_removes_agreed_proposals` (Task 7).
5. **Undoing a merge that has two votes** must restore both "One agent only" proposals with their confidence and provenance intact, so a second merge pass sees the same inputs. Test: `undo_merge_restores_votes_with_provenance` (Task 8).

## Before Task 1

The working tree has uncommitted changes (full-text panel, PDF hash migration `0027`, `StepConfirmBar.tsx`, export panel). Ask the researcher to commit or stash them first. This plan edits the same files and must not mix with that work. Then create a branch: `git checkout -b ai-transparency`.

---

### Task 1: Provenance plumbing (seed, model used, prompt fingerprint)

**Files:**
- Create: `src-tauri/src/transparency.rs` (pure helpers; `mod transparency;` in `lib.rs`)
- Modify: `src-tauri/src/lmstudio.rs` (`ChatRequest`, new `chat_seeded`)
- Modify: `src-tauri/src/suggest.rs`: `screen_article`, `screen_article_fulltext`, `code_article_for_dimensions`, `adjudicate_disagreements`, `consolidate_codes`, `generate_synthesis`
- Modify: `src-tauri/src/lib.rs`: all call sites listed by `grep -n "suggest::\(screen_article\|code_article_for_dimensions\|adjudicate_disagreements\|consolidate_codes\|generate_synthesis\)"`, plus `fn load_seed`
- Create: `src-tauri/migrations/0028_ai_transparency.sql` (this task adds only the `ai_call` table; later tasks append to the same file before anything ships)
- Modify: `src-tauri/src/db.rs`: `insert_ai_call`
- Modify: `src-tauri/Cargo.toml`: `[dev-dependencies] tempfile = "3"`

**Interfaces:**
- Produces:
  - `transparency::prompt_fingerprint(system_prompt: &str) -> String`: first 12 lowercase hex chars of SHA-256 (`sha2` is already a dependency).
  - `suggest::CallMeta { model_requested: String, model_used: String, prompt_hash: String, temperature: f32, seed: Option<i64> }` (`Serialize, Clone, Debug`). `model_used` = `ChatResult.model` when non-empty, else `model_requested`. `prompt_hash` = fingerprint of the **system** message only.
  - `lmstudio::chat_seeded(base_url, model, messages, temperature: Option<f32>, seed: Option<i64>, api_key) -> Result<ChatResult>`. `chat(...)` becomes a wrapper passing `None`.
  - Each of the six suggest functions gains a trailing `seed: Option<i64>` parameter before `api_key` and returns `Result<(T, CallMeta)>`.
  - `fn load_seed(settings: &serde_json::Value) -> Option<i64>`: reads `ai.seed`; `null`, missing or non-integer gives `None`.
  - `ProjectDb::insert_ai_call(&self, purpose: &str, meta: &CallMeta) -> Result<i64>`. `purpose` ∈ `screening_ta | screening_ft | coding | merging | consolidating | synthesis`.
  - Test helper in `db.rs` under `#[cfg(test)]`: `pub(crate) async fn test_db() -> (ProjectDb, tempfile::TempDir)`, which calls `open_or_create` on a temp dir.

Migration, first block:
```sql
CREATE TABLE ai_call (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    purpose         TEXT NOT NULL,
    model_requested TEXT NOT NULL,
    model_used      TEXT NOT NULL,
    prompt_hash     TEXT NOT NULL,
    temperature     REAL NOT NULL,
    seed            INTEGER,
    created_at      TEXT NOT NULL
);
```

- [ ] **Step 1: Write failing tests** in `transparency.rs` and `lmstudio.rs`:
```rust
#[test] fn fingerprint_is_12_hex_and_stable() {
    let a = prompt_fingerprint("You are X"); assert_eq!(a.len(), 12);
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(a, prompt_fingerprint("You are X"));
    assert_ne!(a, prompt_fingerprint("You are Y"));
}
#[test] fn seed_omitted_when_none() {   // lmstudio.rs
    let body = serde_json::to_value(ChatRequest { model: "m", messages: &[], temperature: Some(0.0), max_tokens: None, stream: false, seed: None }).unwrap();
    assert!(body.get("seed").is_none());
    let body = serde_json::to_value(ChatRequest { seed: Some(7), ..same }).unwrap();
    assert_eq!(body["seed"], 7);
}
#[test] fn load_seed_off_by_default() {   // lib.rs tests
    assert_eq!(load_seed(&serde_json::json!({})), None);
    assert_eq!(load_seed(&serde_json::json!({"ai.seed": null})), None);
    assert_eq!(load_seed(&serde_json::json!({"ai.seed": 42})), Some(42));
}
#[tokio::test] async fn insert_ai_call_roundtrip() {   // db.rs
    let (db, _d) = test_db().await;
    let id = db.insert_ai_call("coding", &CallMeta{ model_requested:"a".into(), model_used:"a-q4".into(), prompt_hash:"abc123abc123".into(), temperature:0.2, seed:None }).await.unwrap();
    let used: String = sqlx::query_scalar("SELECT model_used FROM ai_call WHERE id=?").bind(id).fetch_one(&db.pool).await.unwrap();
    assert_eq!(used, "a-q4");
}
```
- [ ] **Step 2:** Run `cd src-tauri && cargo test fingerprint seed_omitted load_seed insert_ai_call`. Expected: compile FAIL (items missing).
- [ ] **Step 3: Implement** the interfaces above. Add `#[serde(skip_serializing_if = "Option::is_none")] seed: Option<i64>` to `ChatRequest`. Callers in `lib.rs` destructure `(value, _meta)` for now. `build_codebook` and `generate_synthesis` keep ignoring meta permanently. Pass `load_seed(&settings)` at every call site.
- [ ] **Step 4:** Run `cargo test`. Expected: all pass. Also `cargo clippy --no-deps 2>&1 | grep -c warning` must not grow.
- [ ] **Step 5: Commit** `feat(ai): record model used, prompt fingerprint, temperature, seed per call`

---

### Task 2: Screening log schema, open events, review status

**Files:**
- Modify: `src-tauri/migrations/0028_ai_transparency.sql` (append)
- Modify: `src-tauri/src/db.rs`: replace `save_ai_screening_proposal`, extend `AiScreeningProposal`, add the functions below
- Modify: `src-tauri/src/transparency.rs`: `review_status`

Migration, appended. This rebuilds the table so `confidence` can be NULL. It backfills `auto_applied` for old rows whose decision is still the one reflected on the article.
```sql
CREATE TABLE ai_screening_proposal_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id INTEGER NOT NULL REFERENCES article(id) ON DELETE CASCADE,
    rater INTEGER NOT NULL DEFAULT 1,
    stage TEXT NOT NULL DEFAULT 'title_abstract',   -- title_abstract | fulltext
    mode TEXT NOT NULL DEFAULT 'screening',          -- screening | purification
    decision TEXT NOT NULL,                          -- include|exclude|uncertain|error|skipped
    criterion_number INTEGER,
    rationale TEXT NOT NULL,
    confidence REAL,
    model_name TEXT,
    ai_call_id INTEGER REFERENCES ai_call(id),
    auto_applied INTEGER NOT NULL DEFAULT 0,
    overridden_at TEXT,
    overridden_to TEXT,                               -- include|exclude|reverted|reset
    run_id TEXT NOT NULL,
    created_at TEXT NOT NULL
);
INSERT INTO ai_screening_proposal_new (id, article_id, rater, decision, criterion_number, rationale, confidence, model_name, run_id, created_at, auto_applied)
SELECT p.id, p.article_id, p.rater, p.decision, p.criterion_number, p.rationale, p.confidence, p.model_name, p.run_id, p.created_at,
       CASE WHEN a.ai_rationale = p.rationale AND
                 ((p.decision='exclude' AND a.status='excluded') OR (p.decision='include' AND a.status='included'))
            THEN 1 ELSE 0 END
FROM ai_screening_proposal p JOIN article a ON a.id = p.article_id;
DROP TABLE ai_screening_proposal;
ALTER TABLE ai_screening_proposal_new RENAME TO ai_screening_proposal;
CREATE INDEX idx_asp_article ON ai_screening_proposal(article_id, stage);

CREATE TABLE ai_screening_open (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    proposal_id INTEGER NOT NULL REFERENCES ai_screening_proposal(id) ON DELETE CASCADE,
    opened_by TEXT NOT NULL,
    via TEXT NOT NULL,          -- detail | decision
    opened_at TEXT NOT NULL
);
CREATE INDEX idx_aso_proposal ON ai_screening_open(proposal_id);
```

**Definition of "opened"** (decided here so the UI and report agree). An open event is recorded (a) each time the researcher opens the AI decision detail dialog for that decision (`via='detail'`), and (b) when the researcher makes any screening decision on that article at that stage, because the decision was on screen in the card (`via='decision'`). Merely scrolling past a card is not an open.

**Interfaces:**
- Produces:
  - `pub struct NewScreeningDecision<'a> { article_id: i64, stage: &'a str, mode: &'a str, decision: &'a str, criterion_number: Option<i64>, rationale: &'a str, confidence: Option<f64>, model_name: Option<&'a str>, ai_call_id: Option<i64>, auto_applied: bool, run_id: &'a str }`
  - `db.save_ai_screening_decision(&NewScreeningDecision<'_>) -> Result<i64>`, which replaces `save_ai_screening_proposal` (delete the old fn).
  - `AiScreeningProposal` gains `stage: String, mode: String, confidence: Option<f64>, auto_applied: bool, overridden_at: Option<String>, overridden_to: Option<String>, ai_call_id: Option<i64>`. Existing list queries filter `stage = 'title_abstract'` where they feed the disagreement banner.
  - `pub struct AiScreeningLogEntry` (`Serialize`): every `AiScreeningProposal` field plus `article_title: String, model_used: Option<String>, prompt_hash: Option<String>, temperature: Option<f64>, seed: Option<i64>, open_count: i64, first_opened_at: Option<String>, last_opened_at: Option<String>, last_opened_by: Option<String>, is_latest: bool, review_status: Option<String>`
  - `db.list_ai_screening_log(article_ids: Option<&[i64]>, stage: Option<&str>) -> Result<Vec<AiScreeningLogEntry>>`. One query that LEFT JOINs `ai_call` and aggregates `ai_screening_open` (COUNT, MIN, MAX). `is_latest` is true when no newer row has the same (article_id, stage). Order: article_id, stage, created_at.
  - `db.latest_ai_screening_decision(article_id, stage) -> Result<Option<AiScreeningProposal>>`
  - `db.record_ai_screening_open(proposal_id: i64, opened_by: &str, via: &str) -> Result<()>`
  - `db.mark_ai_decision_overridden(article_id: i64, stage: &str, human_decision: &str) -> Result<()>`. This acts on the latest row for (article, stage) where `overridden_at IS NULL` and the decision is not error/skipped. It sets `overridden_at`/`overridden_to` only when `human_decision != decision`.
  - `pub struct AttentionCounts { title_abstract_auto: i64, title_abstract_pending: i64, fulltext_auto: i64, fulltext_pending: i64 }` and `db.ai_screening_attention_counts() -> Result<AttentionCounts>`. These count latest rows with status `auto_applied` or `pending`. A pending row counts only while its article is still undecided at that stage: title/abstract means `article.status='retrieved'`; full text means `status='included' AND fulltext_status IS NULL`.
  - `transparency::review_status(decision: &str, auto_applied: bool, overridden: bool, open_count: i64) -> Option<&'static str>`. Returns `None` for `error`/`skipped`. Otherwise, in order of precedence: `"overridden"`, then `"opened"` (open_count > 0), then `"auto_applied"`, else `"pending"`.
  - `pub fn researcher_name() -> String` in `transparency.rs`: env `USER`, else `"researcher"`.

- [ ] **Step 1: Write failing tests** (pure tests in `transparency.rs`, DB tests in `db.rs`):
```rust
#[test] fn status_precedence() {
    assert_eq!(review_status("exclude", true, true, 3), Some("overridden"));
    assert_eq!(review_status("exclude", true, false, 1), Some("opened"));
    assert_eq!(review_status("exclude", true, false, 0), Some("auto_applied"));
    assert_eq!(review_status("uncertain", false, false, 0), Some("pending"));
}
#[test] fn errors_and_skips_have_no_status() {
    assert_eq!(review_status("error", false, false, 0), None);
    assert_eq!(review_status("skipped", false, false, 0), None);
}
#[tokio::test] async fn two_opens_counted_and_untouched_never_opened() // spec 7b check
// insert article A and B, an auto-applied exclude for each; record_ai_screening_open(A) twice;
// list_ai_screening_log(None, None): A.open_count==2, first_opened_at < = last_opened_at, last_opened_by==researcher_name();
// B.open_count==0, B.first_opened_at==None, B.review_status==Some("auto_applied")
#[tokio::test] async fn latest_per_article_stage_wins()
// two title_abstract rows for one article: only the newer has is_latest==true; attention counts count it once
#[tokio::test] async fn override_only_when_decision_differs()
// AI exclude, mark_ai_decision_overridden(.., "exclude") → overridden_at None; then "include" → Some, overridden_to=="include"
#[tokio::test] async fn attention_counts_ignore_articles_no_longer_undecided()
// pending uncertain row on article with status='retrieved' → title_abstract_pending==1; UPDATE status='excluded' → 0
#[tokio::test] async fn migration_backfills_auto_applied()
// covered by opening a DB: insert via raw SQL into a fresh DB is post-migration, so instead assert
// the migration SQL file contains the CASE backfill (string check). Real check is in Task 12 on a copy of a real project.
```
- [ ] **Step 2:** Run `cargo test status_ errors_and_skips two_opens latest_per override_only attention_counts`. Expected: FAIL.
- [ ] **Step 3: Implement** the migration and functions above. Fix the compile errors this causes in `lib.rs` by switching the two `save_ai_screening_proposal` call sites to `save_ai_screening_decision` with `stage:"title_abstract", mode` matching `args.exclude_only`, and the current `auto_applied` semantics. Task 3 reworks that loop properly.
- [ ] **Step 4:** Run `cargo test`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(screening): full screening log schema, open events, derived review status`

---

### Task 3: Complete screening log in both AI runs (spec 1, 3)

**Files:**
- Modify: `src-tauri/src/transparency.rs`: `screening_outcome`
- Modify: `src-tauri/src/lib.rs`: `run_ai_screening` (~3354), `run_ai_fulltext_screening` (~1772)

**Interfaces:**
- Consumes: `save_ai_screening_decision`, `insert_ai_call`, `CallMeta` (Tasks 1–2)
- Produces: `transparency::screening_outcome(decision: &str, confidence: f64, threshold: f64, exclude_only: bool) -> (Option<&'static str>, bool)`. The first value is the article change to apply: `Some("include")`, `Some("exclude")`, `Some("uncertain")` (store rationale only) or `None` (purification leaves the article untouched). The second is `auto_applied`. Rules, which are today's behaviour:
  - exclude with conf ≥ threshold gives `(Some("exclude"), true)` in either mode
  - in exclude_only mode, anything else gives `(None, false)`
  - include with conf ≥ threshold gives `(Some("include"), true)`
  - anything else gives `(Some("uncertain"), false)`

Every article in the loop writes exactly one log row:

| Path | decision | confidence | auto_applied | ai_call_id |
|---|---|---|---|---|
| model returned | model's | model's | per `screening_outcome` | from `insert_ai_call("screening_ta"/"screening_ft")` |
| model error | `error` | NULL | false | NULL; rationale = error text truncated to 500 chars |
| full text: no PDF / extract failed | `skipped` | NULL | false | NULL; rationale `"No PDF attached"` / `"PDF text could not be extracted"` |

Full text also sets `article.ai_rationale/ai_confidence` on confident include and exclude, matching title/abstract. It logs a `review_event` `fulltext_include` with actor `ai` for auto includes, as already done for excludes. `mode` is `"purification"` when `args.exclude_only`, else `"screening"`. Full text is always `"screening"`.

- [ ] **Step 1: Write failing tests:**
```rust
#[test] fn outcome_matches_previous_threshold_rules() {
    assert_eq!(screening_outcome("exclude", 0.90, 0.90, false), (Some("exclude"), true));
    assert_eq!(screening_outcome("exclude", 0.89, 0.90, false), (Some("uncertain"), false));
    assert_eq!(screening_outcome("include", 0.95, 0.90, false), (Some("include"), true));
    assert_eq!(screening_outcome("include", 0.95, 0.90, true),  (None, false));
    assert_eq!(screening_outcome("exclude", 0.95, 0.90, true),  (Some("exclude"), true));
    assert_eq!(screening_outcome("uncertain", 0.99, 0.90, false), (Some("uncertain"), false));
}
```
- [ ] **Step 2:** `cargo test outcome_matches`. Expected: FAIL.
- [ ] **Step 3: Implement** `screening_outcome` and rewrite both loops to call it. Every branch, including the error `continue` and the no-PDF `continue`, must call `save_ai_screening_decision` first. The frontend progress events are unchanged.
- [ ] **Step 4:** `cargo test`, PASS. Then `npm run tauri dev`: screen a 5-article test project where one article has no abstract and the model endpoint is pointed at a dead port for one run. Check `sqlite3 <ws>/project.sqlite "SELECT article_id, stage, decision, auto_applied FROM ai_screening_proposal"` shows a row per article per run.
- [ ] **Step 5: Commit** `fix(screening): log every AI screening outcome incl. full-text includes, unsure, purification, errors`

---

### Task 4: Log reverts, resets, overrides and decision-opens (spec 2, 7b)

**Files:**
- Modify: `src-tauri/src/transparency.rs`: `describe_reverted_decision`
- Modify: `src-tauri/src/lib.rs`: `unscreen_article`, `reset_fulltext_article`, `keep_article`, `exclude_article`, `fulltext_keep_article`, `fulltext_exclude_article`, `accept_ai_screening_proposal`, plus new commands `record_ai_screening_open`, `list_ai_screening_log`, `ai_screening_attention_counts` (register in `generate_handler!`)
- Modify: `src/lib/api.ts`: types and wrappers

**Interfaces:**
- Produces:
  - `transparency::describe_reverted_decision(kind: &str /* "revert"|"reset" */, article_id: i64, prev_status: &str, prev_fulltext: Option<&str>, prev_criterion: Option<i64>, prev_reason: Option<&str>, prev_confidence: Option<f64>, prev_ai_model: Option<&str>) -> (String /*detail*/, Option<String> /*rationale*/)`. Detail format: `Reverted title/abstract decision on article 12 (was: excluded, criterion 4, AI confidence 93%, model qwen3-8b)`. For a reset, start with `Reset full-text decision on article 12 (was: …)`. Leave out the parts that are None. The rationale is the previous reason verbatim.
  - The existing review_event actions get two new ones: `titleabstract_revert` and `fulltext_reset` (actor `human`). Both are written **before** the db call that clears the fields. Previous values come from `get_article_by_id` + `fulltext_status_map` / the latest AI log row.
  - Override and open wiring. `stage` is `title_abstract` for the first two commands and `fulltext` for the next two:
    - `keep_article` / `exclude_article`: `record_open(latest_ta, "decision")`, then `mark_ai_decision_overridden(id, "title_abstract", "include"|"exclude")`
    - `fulltext_keep_article` / `fulltext_exclude_article`: the same, with stage `fulltext`
    - `unscreen_article`: override both stages with `"reverted"`
    - `reset_fulltext_article`: override fulltext with `"reset"`
    - `accept_ai_screening_proposal`: record an open (`decision`). There is no override because the decision matches.
  - Commands: `record_ai_screening_open(proposal_id: i64)` (via `detail`, by `researcher_name()`), `list_ai_screening_log(article_ids: Option<Vec<i64>>, stage: Option<String>) -> Vec<AiScreeningLogEntry>`, `ai_screening_attention_counts() -> AttentionCounts`.
  - TS: `export type ReviewStatus = "auto_applied" | "opened" | "overridden" | "pending"`. `interface AiScreeningLogEntry` mirrors the Rust struct. `interface AttentionCounts`. Wrappers `api.recordAiScreeningOpen(id)`, `api.listAiScreeningLog(ids?, stage?)`, `api.aiScreeningAttentionCounts()`. `AiScreeningProposal.confidence` becomes `number | null`.

- [ ] **Step 1: Write failing tests:**
```rust
#[test] fn revert_detail_keeps_old_ai_reason_and_confidence() {
    let (d, r) = describe_reverted_decision("revert", 12, "excluded", None, Some(4), Some("Off-topic: K-12 sample"), Some(0.93), Some("qwen3-8b"));
    assert_eq!(d, "Reverted title/abstract decision on article 12 (was: excluded, criterion 4, AI confidence 93%, model qwen3-8b)");
    assert_eq!(r.as_deref(), Some("Off-topic: K-12 sample"));
}
#[test] fn reset_detail_for_fulltext() {
    let (d, _) = describe_reverted_decision("reset", 3, "excluded", Some("excluded"), None, None, None, None);
    assert_eq!(d, "Reset full-text decision on article 3 (was: excluded at full text)");
}
```
- [ ] **Step 2:** `cargo test revert_detail reset_detail`. Expected: FAIL.
- [ ] **Step 3: Implement.** Then run `npx tsc --noEmit`, which must be clean (it flags any UI code that assumed `confidence: number`).
- [ ] **Step 4:** `cargo test` PASS. Manually, in dev: undo an AI exclusion and reset a full-text decision, then confirm both rows appear in `review_event` with the old reason in `rationale`. **This is spec 2's check.**
- [ ] **Step 5: Commit** `feat(screening): log reverts/resets with prior AI reason; record opens and overrides`

---

### Task 5: Review status in the screening UI (spec 3, 7, 7b)

**Files:**
- Create: `src/components/AiDecisionStatus.tsx`: `ReviewStatusBadge` + `AiDecisionDetailDialog`
- Modify: `src/views/arranging/ScreeningPanel.tsx`: load the log for the batch; show the badge; replace the inline rationale modal with `AiDecisionDetailDialog`
- Modify: `src/views/arranging/FullTextScreeningPanel.tsx`: show the full-text AI decision (from the log, `stage="fulltext"`) with badge and dialog. The "AI logic" line reads from the log, not `ai_rationale`.

**Interfaces:**
- Consumes: `api.listAiScreeningLog`, `api.recordAiScreeningOpen`, `AiScreeningLogEntry`, `ReviewStatus` (Task 4)
- Produces:
  - `ReviewStatusBadge({ status }: { status: ReviewStatus })` with labels from Global Constraints. Colours: auto_applied = accent, opened = paper/neutral, overridden = warn, pending = warn outline.
  - `AiDecisionDetailDialog({ entry, onClose }: { entry: AiScreeningLogEntry; onClose: () => void })`. On mount it calls `api.recordAiScreeningOpen(entry.id)` exactly once. React StrictMode double-mount must not double count: guard with a `useRef` keyed on `entry.id`. It shows decision, criterion, reason (existing `RationaleBody`), confidence, model used, prompt fingerprint, temperature, "Applied automatically" if so, and "Opened N times · first … · last …" or **Never opened** (that state is only visible before this open lands, so render from the entry as loaded). The footer reads "Opening is logged. Reeves records that this decision was opened, not that it was assessed."
  - Card: the badge sits next to the AI line. Cards for articles decided by a human with no AI row show nothing new.

- [ ] **Step 1:** Implement the components (the repo has no frontend test runner, so verification is by build plus a manual check).
- [ ] **Step 2:** Run `npm run build`. Expected: exit 0, no TS errors.
- [ ] **Step 3: Manual check (spec 7 / 7b):** In `npm run tauri dev`, make one example of each status:
  - auto-applied exclude, left untouched: **Applied automatically**, Never opened
  - another opened twice: **Opened, left unchanged**, 2 opens
  - one kept against an AI exclude: **Overridden**
  - an uncertain one: **Pending**

  Grep the UI for wording: `grep -rniE "\b(read|reviewed)\b" src/components/AiDecisionStatus.tsx`. Expected: no hits.
- [ ] **Step 4: Commit** `feat(screening-ui): review status badge and logged decision detail`

---

### Task 6: Non-blocking reminders (spec 8)

**Files:**
- Create: `src/components/AiReviewReminder.tsx`
- Modify: `src/views/arranging/ArrangingStage.tsx` (advance to assessing, ~286)
- Modify: `src/components/StepConfirmBar.tsx` (confirming the `screening` / `fulltext` steps)
- Modify: `src/views/assessing/ExportPanel.tsx` (`run(kind)`)
- Modify: `src/views/assessing/AssessingStage.tsx` and `src/App.tsx`: new prop `onOpenArranging(tab: "screening" | "fulltext")`, which saves `last_tab.arranging` and calls `setRoute("arranging")`
- Modify: `src-tauri/src/lib.rs`: command `log_ai_reminder_choice(context: String, choice: String, auto_count: i64, pending_count: i64)` writes review_event stage `arranging`, action `ai_reminder_open_now` | `ai_reminder_continue`, detail `"Before <context>: <auto> applied automatically, <pending> pending"`, actor `human`

**Interfaces:**
- Produces: `useAiReviewReminder(): { guard: (context: string, proceed: () => void | Promise<void>, onOpen: (tab: "screening" | "fulltext") => void) => Promise<void>; element: JSX.Element | null }`. `guard` fetches `aiScreeningAttentionCounts()`:
  - if the total is 0, or the fetch throws, it calls `proceed()` immediately
  - otherwise it shows the dialog: "N AI screening decisions were applied automatically and haven't been opened; M are pending."
  - **Open them now** logs `open_now`, then calls `onOpen` with `"screening"` when there are title/abstract counts, else `"fulltext"`
  - **Continue anyway** logs `continue`, then calls `proceed()`
  - A failure to log must never stop `proceed()` (catch and ignore). Closing the dialog with Esc or a backdrop click counts as neither and does not proceed.
  - `context` values: `"moving to Stage 3"`, `"confirming title/abstract screening"`, `"confirming full-text screening"`, `"exporting <kind>"`.

- [ ] **Step 1:** Implement the Rust command, then add a test for the detail string:
```rust
#[test] fn reminder_detail_text() { assert_eq!(reminder_detail("exporting docx", 3, 2), "Before exporting docx: 3 applied automatically, 2 pending"); }
```
- [ ] **Step 2:** `cargo test reminder_detail`. Expected: FAIL, then implement `fn reminder_detail(context:&str, auto:i64, pending:i64)->String` and PASS.
- [ ] **Step 3:** Implement the hook and wire the four call sites. The existing checkpoint `allChecked` gating is unchanged; the reminder adds no new disabling.
- [ ] **Step 4:** `npm run build` exits 0. Manual check:
  - with zero counts, advancing and exporting show no dialog
  - with counts > 0, the dialog appears, and **Continue anyway** proceeds every time
  - **Open them now** lands on the screening tab
  - both choices appear in `review_event`
- [ ] **Step 5: Commit** `feat(reminders): non-blocking reminder for unopened/pending AI screening decisions`

---

### Task 7: Coding — confidence, both reasons, suggestion mode, accept logging (spec 4, 9, 10)

**Files:**
- Modify: `src-tauri/migrations/0028_ai_transparency.sql` (append)
- Modify: `src-tauri/src/suggest.rs`: `Suggestion.confidence`, coding output-format line
- Modify: `src-tauri/src/transparency.rs`: `resolve_rater_codes`, `coding_disposition`
- Modify: `src-tauri/src/db.rs`: `save_ai_coding`, `save_ai_coding_proposal` (new signature), `CodingRow`/`AiCodingProposal` fields, `revert_ai_coding`
- Modify: `src-tauri/src/lib.rs`: `run_ai_coding` resolve block (~3040–3115), `accept_ai_coding_proposal`, `reject_all_ai_coding_proposals_for_article` (now logs), new `accept_agreed_ai_coding_proposals`
- Modify: `src/lib/api.ts`, `src/views/arranging/CodingPanel.tsx`, `src/components/AiCodingModal.tsx`, `src/views/Settings.tsx` (mode toggle)

Migration, appended:
```sql
ALTER TABLE coding ADD COLUMN rationale2 TEXT;
ALTER TABLE coding ADD COLUMN model_name2 TEXT;
ALTER TABLE coding ADD COLUMN confidence REAL;
ALTER TABLE coding ADD COLUMN confidence2 REAL;
ALTER TABLE coding ADD COLUMN ai_call_id INTEGER;
ALTER TABLE coding ADD COLUMN ai_call_id2 INTEGER;
ALTER TABLE ai_coding_proposal ADD COLUMN agreement TEXT NOT NULL DEFAULT 'one';
ALTER TABLE ai_coding_proposal ADD COLUMN confidence REAL;
ALTER TABLE ai_coding_proposal ADD COLUMN ai_call_id INTEGER;
ALTER TABLE ai_coding_proposal ADD COLUMN rationale2 TEXT;
ALTER TABLE ai_coding_proposal ADD COLUMN model_name2 TEXT;
ALTER TABLE ai_coding_proposal ADD COLUMN confidence2 REAL;
ALTER TABLE ai_coding_proposal ADD COLUMN ai_call_id2 INTEGER;
-- Existing projects keep today's behaviour; a brand-new DB has no project row yet at migration time.
INSERT OR IGNORE INTO settings (key, value, updated_at)
SELECT 'coding.agreed_codes_mode', '"auto"', datetime('now') WHERE EXISTS (SELECT 1 FROM project WHERE id = 1);
```

**Interfaces:**
- Produces:
  - `Suggestion { …, #[serde(default)] confidence: Option<f64> }`. Values outside 0–1 are clamped. Values > 1 and ≤ 100 are divided by 100.
  - Coding output-format line becomes `Each value is a JSON array of objects with keys: code (string), rationale (string, one sentence), confidence (number 0.0–1.0, your own estimate of how well the text supports this code), is_new (boolean).` Confidence is never used to filter or rank codes.
  - `pub struct AiCodeVote { rationale: Option<String>, model_name: Option<String>, confidence: Option<f64>, ai_call_id: Option<i64> }` (`Clone, Debug`), in `transparency.rs`. `db.rs` imports it.
  - `pub struct ResolvedCode { dimension: String, code: String /* canonical_theme applied */, r1: Option<AiCodeVote>, r2: Option<AiCodeVote> }`
  - `transparency::resolve_rater_codes(r1: &[DimensionCoding], r1_vote: impl Fn(&Suggestion) -> AiCodeVote, r2: Option<&[DimensionCoding]>, r2_vote: impl Fn(&Suggestion) -> AiCodeVote) -> Vec<ResolvedCode>`. It moves today's matching (`themes_match`, `canonical_theme`) out of `lib.rs` unchanged; move those two helpers into `transparency.rs` too.
  - `pub enum Disposition { ApplyCoding { source: &'static str }, Propose { agreement: &'static str, rater: i64 } }` and `transparency::coding_disposition(mode: &str, dual: bool, rc: &ResolvedCode) -> Disposition`:

    | mode | rater setup | code | Disposition |
    |---|---|---|---|
    | auto | dual | both agents | `ApplyCoding{"model-ai-consensus"}` |
    | auto | dual | rater 1 only | `Propose{"one",1}` |
    | auto | dual | rater 2 only | `Propose{"one",2}` |
    | auto | single | any | `ApplyCoding{"model-ai-consensus"}` |
    | suggest | dual | both agents | `Propose{"both",1}` |
    | suggest | any | one agent | `Propose{"one",rater}` |
    | suggest | single | any | `Propose{"one",1}` |

  - `db.save_ai_coding(article_id, dimension, code, source: &str, first: &AiCodeVote, second: Option<&AiCodeVote>) -> Result<i64>`
  - `db.save_ai_coding_proposal(article_id, dimension, code, rater: i64, agreement: &str, first: &AiCodeVote, second: Option<&AiCodeVote>, run_id: &str) -> Result<i64>`. This replaces the old 7-arg fn; update `undo_adjudication`.
  - `CodingRow` and `AiCodingProposal` gain `rationale2, model_name2, confidence, confidence2, model_used, model_used2, prompt_hash, temperature` (the last four via LEFT JOIN `ai_call`), and proposals gain `agreement`.
  - `run_ai_coding` reads `coding.agreed_codes_mode` (missing → `"suggest"`). It inserts one `ai_call` per rater per article (`purpose "coding"`) and builds votes with that id.
  - `accept_ai_coding_proposal` writes source `"model-ai-accepted"` with both votes when `agreement='both'`, and logs review_event `accept_code`, detail `Accepted code “X” in Dim (agreed by both agents | one agent only, rater N)`, actor `human`.
  - `accept_agreed_ai_coding_proposals(article_ids: Option<Vec<i64>>) -> usize` accepts every `agreement='both'` proposal (optionally scoped) through the same path, one log row each.
  - `revert_ai_coding` also deletes proposals of every agreement (today it deletes all proposals already; keep that and add a test).
  - UI:
    - Suggestions show the agreement label and per-agent reasons, each with a confidence chip (`72%`, tooltip with the Global Constraints label).
    - Agreed applied codes show both reasons.
    - The proposal review header gets an **Accept all agreed (N)** button, which only appears in suggest mode when N > 0.
    - Settings → AI raters gets a radio: "Agreed codes: Apply automatically / Show as suggestions".

- [ ] **Step 1: Write failing tests:**
```rust
#[test] fn suggestion_mode_saves_nothing_to_coding() {
    let rc = ResolvedCode{ dimension:"Method".into(), code:"Survey".into(), r1:Some(v()), r2:Some(v()) };
    assert!(matches!(coding_disposition("suggest", true, &rc), Disposition::Propose{agreement:"both", rater:1}));
    let single = ResolvedCode{ r2:None, ..rc.clone() };
    assert!(matches!(coding_disposition("suggest", false, &single), Disposition::Propose{agreement:"one", rater:1}));
}
#[test] fn auto_mode_matches_previous_behaviour() {
    // dual agreed → ApplyCoding consensus; dual r1-only → Propose one/1; dual r2-only → Propose one/2; single → ApplyCoding
}
#[test] fn agreed_code_keeps_both_votes() {
    // r1 "Structural equation modeling" conf .8 rationale "A"; r2 "structural equation modelling" conf .6 rationale "B"
    // resolve_rater_codes → exactly one ResolvedCode; r1.rationale=="A", r2.rationale=="B", code=="Structural equation modeling"
}
#[test] fn confidence_parsed_and_clamped() {
    // parse_dimension_codings on {"Method":[{"code":"Survey","rationale":"x","confidence":85,"is_new":false}]} → Some(0.85)
    // missing confidence → None
}
#[tokio::test] async fn accept_logs_review_event()
#[tokio::test] async fn revert_removes_agreed_proposals()
#[tokio::test] async fn existing_project_migration_sets_auto()   // open DB, insert project row, re-run the INSERT OR IGNORE statement text → value "auto"; fresh test_db() → setting absent before project row exists
```
- [ ] **Step 2:** `cargo test suggestion_mode auto_mode agreed_code confidence_parsed accept_logs revert_removes existing_project`. Expected: FAIL.
- [ ] **Step 3: Implement** the backend, then the UI.
- [ ] **Step 4:** `cargo test` PASS, `npm run build` exit 0. Manual check:
  - suggest mode: an AI run leaves `SELECT COUNT(*) FROM coding WHERE source LIKE 'model-ai%'` unchanged until you accept
  - **Accept all agreed** moves them over and writes `accept_code` rows
  - auto mode on a copy of an existing project gives the same consensus count as `main` on the same copy (same models, temperature 0)
- [ ] **Step 5: Commit** `feat(coding): confidence per suggestion, both agents' reasons, suggestion mode, accept logging`

---

### Task 8: Merging agent — rename, model choice, warning, provenance, logged undo (spec 5, 11)

**Files:**
- Modify: `src-tauri/migrations/0028_ai_transparency.sql` (append: `ALTER TABLE ai_adjudication ADD COLUMN` for each of `ai_call_id INTEGER`, `confidence1 REAL`, `confidence2 REAL`, `ai_call_id1 INTEGER`, `ai_call_id2 INTEGER`; `ALTER TABLE ai_consolidation ADD COLUMN ai_call_id INTEGER;`)
- Modify: `src-tauri/src/lib.rs`: Phase 3/4 of `run_ai_coding`, `undo_adjudication`, `undo_consolidation`, `AiCodingResult` doc comments
- Modify: `src-tauri/src/suggest.rs`: adjudication prompt wording (see Task 9 for the split)
- Modify: `src/components/AiCodingModal.tsx`, `src/views/arranging/CodingPanel.tsx`, `src/lib/api.ts` (comments), `src/views/Settings.tsx:473` hint, `src-tauri/src/draft.rs` (report wording, done in Task 10)

**Interfaces:**
- Produces:
  - Settings keys `ai_agents.merging_model` and `ai_agents.consolidating_model` (blank means `lmstudio.chat_model`). `run_ai_coding` uses them in Phase 3 and Phase 4, and inserts `ai_call` rows `purpose "merging"` / `"consolidating"`. The ids are stored on `ai_adjudication.ai_call_id` / `ai_consolidation.ai_call_id` and on the merged `coding.ai_call_id`.
  - Phase 3 input filter: only proposals with `agreement = 'one'` from this run.
  - `insert_adjudication` also stores each input proposal's `confidence` and `ai_call_id` (`confidence1/2`, `ai_call_id1/2`). `AiAdjudication` gains these four fields plus `ai_call_id`.
  - `undo_adjudication` restores both proposals via the new `save_ai_coding_proposal` (agreement `"one"`, rationale/model/confidence/ai_call_id from the adjudication row). It logs review_event `undo_merge`, detail `Undid merge of “L1” + “L2” → “Canonical” in Dim`, rationale = the merging agent's reason.
  - `undo_consolidation` logs `undo_consolidation`, detail `Undid consolidation “Variant” → “Canonical” in Dim (N codes)`.
  - `transparency::self_preference_warning(merging_or_consolidating: &str, rater1: &str, rater2: Option<&str>) -> bool`. True when a non-blank merging or consolidating model equals either rater model (case-insensitive, trimmed).
  - Visible strings:
    - `"adjudicating"` progress label becomes "Merging leftover codes that mean the same thing"
    - "Resolved by adjudicator" becomes "Merged by the merging agent"
    - the CodingPanel section title becomes "Merged by the merging agent (undo to send back to you)"
    - the tooltip becomes "Leftover codes the merging agent found to mean the same thing"
    - the Settings hint drops "for adjudication" in favour of "for you to resolve"
  - AiCodingModal: under the coder pickers, two selects ("Merging agent model", "Consolidating agent model") default to the main chat model and persist on change via `api.setSetting`. When `self_preference_warning` applies (mirror the logic in TS), show the warning text inline in warn colour. The Run button is never disabled by it.

- [ ] **Step 1: Write failing tests:**
```rust
#[test] fn warning_only_on_match() {
    assert!(self_preference_warning("qwen3-8b", "Qwen3-8B ", None));
    assert!(self_preference_warning("gemma", "qwen", Some("gemma")));
    assert!(!self_preference_warning("gemma", "qwen", Some("llama")));
    assert!(!self_preference_warning("", "qwen", None));
}
#[tokio::test] async fn undo_merge_restores_votes_with_provenance()
// insert adjudication with confidence1=.7, ai_call_id1=Some(5); undo → two proposals agreement 'one', one with confidence .7 & ai_call_id 5; review_event 'undo_merge' exists
```
- [ ] **Step 2:** `cargo test warning_only undo_merge`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** `cargo test` PASS; `npm run build` exit 0; then the spec 5 check:
```bash
grep -rni "adjudicat" src README.md | grep -viE "^\S+:\s*(//|\*|/\*)" | grep -viE "AiAdjudication|adjudications|listAdjudications|undoAdjudication|isAdjudicated|model-ai-adjudicated|\"adjudicating\"|adjudicated:|adjudicator_rationale"
```
Expected: no output. Remaining hits must be identifiers, not rendered text. Manual check: undo a merge in dev and see `undo_merge` in `review_event`.
- [ ] **Step 5: Commit** `feat(coding): merging agent naming, selectable models with self-preference warning, logged undo`

---

### Task 9: Editable merging, consolidating and synthesis prompts (spec 12)

**Files:**
- Modify: `src-tauri/src/suggest.rs`: split the three system prompts
- Modify: `src-tauri/src/lib.rs`: pass the settings through; new command `get_default_agent_prompts() -> { merging, consolidating, synthesis }`
- Modify: `src/views/Settings.tsx` (AI raters tab: new "Other agents" block), `src/lib/useAppState.ts` (`AppSettings` keys, default `""`)

**Interfaces:**
- Produces, in `suggest.rs`:
  - `pub const DEFAULT_MERGING_INSTRUCTIONS: &str` / `MERGING_OUTPUT_FORMAT: &str`
  - `pub const DEFAULT_CONSOLIDATING_INSTRUCTIONS` / `CONSOLIDATING_OUTPUT_FORMAT`
  - `pub const DEFAULT_SYNTHESIS_INSTRUCTIONS` / `SYNTHESIS_OUTPUT_FORMAT`
  - `fn agent_prompt(custom: &str, default: &str, format: &str) -> String`, which returns `format!("{}\n\n{}", instructions, format)` with blank `custom` falling back to `default`
  - Split points (the format part is everything from these words to the end of today's string):
    - merging: `"Respond ONLY with a JSON array"`
    - consolidating: `"Respond ONLY with a JSON array"`
    - synthesis: `"Use markdown: a '##' heading per dimension"`
  - Separator rule: keep today's exact whitespace. If today's text joins with `"\n\n"` use that, otherwise adjust `agent_prompt`'s separator per agent. The legacy-equality test decides.
  - The merging default changes one phrase from today's text: `"You are an independent adjudicator for a systematic literature review."` becomes `"You are an independent merging agent for a systematic literature review."` (Global Constraints). Everything else is byte-identical.
  - `adjudicate_disagreements`, `consolidate_codes` and `generate_synthesis` gain `instructions: &str` (the setting value) before `temperature`.
  - Settings UI: three TextAreas, each with placeholder = default text, a **Reset to default** button (shown when the value is non-blank and differs from the default; it sets the setting to `""`) and a read-only "Output format (fixed)" disclosure showing the format text. Defaults come from `get_default_agent_prompts`; the TS side does not duplicate them.

- [ ] **Step 1: Write failing tests:**
```rust
const LEGACY_CONSOLIDATING: &str = "<paste today's consolidate_codes system string verbatim>";
const LEGACY_SYNTHESIS: &str = "<paste today's generate_synthesis system string verbatim>";
const LEGACY_MERGING: &str = "<paste today's adjudicate_disagreements system string verbatim, with only the adjudicator→merging agent phrase swapped>";
#[test] fn defaults_reproduce_legacy_prompts() {
    assert_eq!(agent_prompt("", DEFAULT_CONSOLIDATING_INSTRUCTIONS, CONSOLIDATING_OUTPUT_FORMAT), LEGACY_CONSOLIDATING);
    assert_eq!(agent_prompt("", DEFAULT_SYNTHESIS_INSTRUCTIONS, SYNTHESIS_OUTPUT_FORMAT), LEGACY_SYNTHESIS);
    assert_eq!(agent_prompt("", DEFAULT_MERGING_INSTRUCTIONS, MERGING_OUTPUT_FORMAT), LEGACY_MERGING);
}
#[test] fn custom_instructions_keep_fixed_format() {
    let p = agent_prompt("Be strict.", DEFAULT_MERGING_INSTRUCTIONS, MERGING_OUTPUT_FORMAT);
    assert!(p.starts_with("Be strict.")); assert!(p.ends_with(MERGING_OUTPUT_FORMAT));
}
#[test] fn blank_custom_uses_default() { assert_eq!(agent_prompt("  \n", "D", "F"), agent_prompt("", "D", "F")); }
```
Copy the legacy strings from `git show HEAD:src-tauri/src/suggest.rs` **before** editing.
- [ ] **Step 2:** `cargo test defaults_reproduce custom_instructions blank_custom`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** `cargo test` PASS; `npm run build` exit 0. Manual check:
  - edit the merging prompt, run coding, and confirm `ai_call.prompt_hash` for `purpose='merging'` differs from the previous run
  - Reset, run again, and confirm the hash returns to the earlier value
- [ ] **Step 5: Commit** `feat(settings): editable merging/consolidating/synthesis instructions with fixed output format`

---

### Task 10: Decision report (spec 3, 7, 7b, 9, 13)

**Files:**
- Modify: `src-tauri/src/draft.rs`: `generate_decision_report`
- Modify: `src-tauri/src/transparency.rs`: `status_label`, `opens_cell`

**Interfaces:**
- Consumes: `list_ai_screening_log`, extended `CodingRow`, `AiAdjudication.ai_call_id`, `AiConsolidation.ai_call_id` (fetch provenance via one `db.list_ai_calls() -> HashMap<i64, AiCallRow>`; add it to `db.rs`).
- Produces:
  - `transparency::status_label(s: Option<&str>) -> &'static str` maps `auto_applied→"Applied automatically"`, `opened→"Opened, left unchanged"`, `overridden→"Overridden"`, `pending→"Pending"`, `None→"—"`.
  - `transparency::opens_cell(count: i64, first: Option<&str>, last: Option<&str>) -> String` gives `"Never opened"` for 0, `"1 (2026-10-26 14:02)"` for 1 and `"3 (first 2026-10-26 14:02, last 2026-10-27 09:15)"` for more (timestamps trimmed to 16 chars, `T` → space).
  - Report changes:
    - **§2 Screening decisions**: add a **Review status** column for articles with an AI decision.
    - **New §2a "AI screening log"**: every row of the log (not only the latest). Columns: Article · Stage · Mode · AI decision · Criterion · Confidence · Model used · Prompt fingerprint · Temperature · Applied automatically (Yes/No) · Review status · Opened. Below it, the seed line and the Global Constraints repeatability sentence.
    - **New §2b "Applied automatically"**: bulleted list of latest auto-applied decisions with their status, for example `"Excluded — <title> (93%, Never opened)"`.
    - **§3 Coding**: add columns Confidence (rater 1 / rater 2), Reasons (both, `R1: … / R2: …`), Model used, Prompt fingerprint, Temperature.
    - **§4**: retitle to "Merging agent and consolidation". The merge table gains Reason · Model used · Prompt fingerprint · Temperature, and the consolidation table gains Model used · Prompt fingerprint · Temperature. Drop the word "adjudicat" from all text.
    - **§5**: the intro sentence lists the new actions (reverts, resets, accepted and rejected suggestions, undone merges, reminder choices).
    - A confidence note under §3: "Confidence is the model's own estimate, not a measured probability."

- [ ] **Step 1: Write failing tests:**
```rust
#[test] fn opens_cell_formats() {
    assert_eq!(opens_cell(0, None, None), "Never opened");
    assert_eq!(opens_cell(2, Some("2026-10-26T14:02:11Z"), Some("2026-10-27T09:15:00Z")), "2 (first 2026-10-26 14:02, last 2026-10-27 09:15)");
}
#[test] fn status_labels() { assert_eq!(status_label(Some("opened")), "Opened, left unchanged"); assert_eq!(status_label(None), "—"); }
#[tokio::test] async fn report_lists_ai_log_and_auto_applied() {
    // test_db + 2 articles + ai_call + two auto-applied excludes; open one twice
    // let md = generate_decision_report(&db).await.unwrap();
    // assert md contains "## 2a. AI screening log", "Never opened", "2 (first", the prompt hash, "## 2b. Applied automatically"
    // assert !md.to_lowercase().contains("adjudicat"); assert !md.contains("reviewed"); assert!(!md.contains("reproducible"))
}
```
- [ ] **Step 2:** `cargo test opens_cell status_labels report_lists`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** `cargo test` PASS.
- [ ] **Step 5: Commit** `feat(report): AI screening log, review status, opens, provenance for coding/merges/consolidations`

---

### Task 11: Honest wording (spec 6) and seed setting UI (spec 13)

**Files:**
- Modify: `README.md`: rewrite the intro, Prerequisites (3), "What's in…" and "Network policy"; delete "What's coming next"
- Modify: `src/views/ProjectPicker.tsx:115-116`, `src/views/Settings.tsx:241, :511` and the Privacy tab (add a "What goes over the network" block plus a Seed field), `src-tauri/tauri.conf.json:37-38`, `src/components/StatusBar.tsx:113` (`"Local server offline"` → `"Model server offline"`)
- Modify: `src/lib/useAppState.ts`: `"ai.seed": number | null`, default `null`

**Exact copy** (use verbatim; the README may add headings around it):
- Welcome screen paragraph: "Systematic literature reviews following the SPAR-4-SLR protocol and PRISMA 2020. Your project lives in a folder on your computer, and usage statistics stay in that folder. Some features go online: reference lookups, open-access PDF searches, the database browser, and any AI model you connect that runs on another computer."
- Settings About description and `shortDescription`: "Systematic literature reviews following SPAR-4-SLR and PRISMA 2020."
- `longDescription`: "Reeves is a desktop application for systematic literature reviews following the SPAR-4-SLR protocol and PRISMA 2020. Projects and usage statistics are stored locally. Reference lookups, open-access PDF searches and the built-in database browser use the internet, and article text is sent to whichever AI model server you configure."
- What goes out, as a list for both the README and Settings → Privacy:
  - **Crossref** (`api.crossref.org`): the DOI, when you look up or enrich a reference.
  - **Unpaywall** (`api.unpaywall.org`) and the publisher sites it points to: the DOI and a contact address, when you search for open-access PDFs.
  - **Database browser and "Find online"**: the pages you open, as in any browser.
  - **Your AI model server**: titles, abstracts, full-text excerpts, codes and prompts. With LM Studio on this Mac nothing leaves the computer. A remote server address sends this content to that server.
  - **Stays local**: the project database, PDFs, drafts, backups, and usage statistics (telemetry). Reeves sends no usage data anywhere.
- Endpoint description (Settings:241): "Reeves talks to any OpenAI-compatible model server. Tested with LM Studio on macOS. Ollama should work but is untested. A remote server receives the article text Reeves sends for screening and coding."
- README platform line: "Tested on macOS with LM Studio. Ollama should work but is untested. Other platforms are untested."
- Seed field (Settings → AI raters, under temperatures): label "Seed (optional)". Empty means off. Hint: "Sent to model servers that support it. Recorded in the decision report to help others re-run the analysis; model outputs may still vary between runs."
- README: remove claims about the "network allowlist viewer", "refuses to connect to anything that isn't 127.0.0.1", "LanceDB", "Pandoc", "WoS / EBSCO / Scopus API connectors" and "foundation slice / Session N". Replace the feature list with a short current-feature list taken from the stage names in `src/views/StageHub.tsx`.

- [ ] **Step 1:** Apply the copy.
- [ ] **Step 2: Verify.** Run:
```bash
grep -rniE "leaves your laptop|own machine|only to a local|refuses to connect|127\.0\.0\.1. or|coming next|LanceDB|Pandoc|allowlist|reproducib|exactly repeat" README.md src src-tauri/tauri.conf.json
```
Expected: no output (placeholder URLs like `http://127.0.0.1:1234/v1` in inputs are fine; the pattern above doesn't match them). `npm run build` exits 0.
- [ ] **Step 3: Commit** `docs: state plainly what goes online; tested platforms; seed setting`

---

### Task 12: End-to-end verification against every spec check

**Files:** none (verification only). Use a **copy** of a real project folder plus a fresh 6-article test project.

- [ ] **Step 1:** `cd src-tauri && cargo test` (all pass) and `npm run build` (exit 0). Paste both outputs into the PR description.
- [ ] **Step 2:** Run the spec checks in `npm run tauri dev`, recording each result as pass or fail:
  1. Screen the test project (T/A, then full text, then a purification pass, plus one run against a stopped model server). `SELECT article_id, stage, mode, decision FROM ai_screening_proposal` has ≥1 row per article per run.
  2. Undo an AI exclusion and reset a full-text decision. Both are in report §5 with the old reason.
  3. Open the real-project copy. Every article with `ai_rationale` and a matching decision shows **Applied automatically**. Re-screening the copy auto-applies the same set as `main` (same model, temperature 0).
  4. An agreed code shows R1 and R2 reasons, and accepting a suggestion writes `accept_code`.
  5. The grep from Task 8 is empty, and an undone merge is in §5.
  6. The grep from Task 11 is empty; read the README and welcome screen yourself.
  7. One example of each of the four statuses appears in both the app and the report.
  8. 7b: one decision opened twice shows `2 (first …, last …)`, and an untouched one shows "Never opened".
  9. Reminder: absent with zero counts, present otherwise, and **Continue anyway** always proceeds.
  10. Each coding suggestion shows a confidence with the estimate note.
  11. Suggest mode adds no AI coding rows before accept; auto mode matches `main`.
  12. The chosen merging model appears in `ai_call.model_requested` for `purpose='merging'`, and the warning appears only on a match.
  13. Report §2a, §3 and §4 show model, fingerprint and temperature for every row.
- [ ] **Step 3:** Use superpowers:requesting-code-review on the branch, then superpowers:finishing-a-development-branch.
