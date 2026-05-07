# Stage Tab Progression Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add numbered + check + dim-todo progression cues to every stage's tab strip so users can see at a glance which substep is current, which are done, and which are still ahead — without losing the ability to click any tab freely.

**Architecture:** A reusable `<ProgressionTabStrip>` component renders tabs with status-aware badges. A `useTabProgress` hook converts data signals + manual flags into a per-tab status map. Stage views supply stage-specific predicates and load the small extra data they need. Manual completion flags persist in the existing `settings` table via the existing `set_setting` / `get_settings` commands.

**Tech Stack:** React 19 + TypeScript, Tailwind CSS + CSS variables, Framer Motion (existing layoutId underline pattern), Rust + Tauri v2, SQLx + SQLite.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src-tauri/src/db.rs` | Extend `AssemblingState` with `has_acquisition_saved`, `has_unresolved_clusters`; add `ArrangingProgress`, `AssessingProgress` structs + queries |
| Modify | `src-tauri/src/lib.rs` | Add `arranging_progress` and `assessing_progress` Tauri commands; register them; export the structs from `db` |
| Modify | `src/lib/api.ts` | Add `ArrangingProgress`, `AssessingProgress` types and the two methods; extend `AssemblingState` shape |
| Create | `src/components/ProgressionTabStrip.tsx` | Pure render: badges + icons + labels + active underline |
| Create | `src/components/MarkStepComplete.tsx` | Inline footer link that toggles a settings flag |
| Create | `src/lib/useTabProgress.ts` | Hook: `(currentTab, predicates, manualFlags) → Record<TabId, "done"|"active"|"todo">` |
| Modify | `src/views/assembling/AssemblingStage.tsx` | Use `<ProgressionTabStrip>` + `useTabProgress`; load manual flags |
| Modify | `src/views/arranging/ArrangingStage.tsx` | Same wiring with `arranging_progress` |
| Modify | `src/views/assessing/AssessingStage.tsx` | Same wiring with `assessing_progress` |
| Modify | `src/views/assembling/JournalScopePanel.tsx` | Append `<MarkStepComplete tab="journals">` |
| Modify | `src/views/assembling/ArticleLibrary.tsx` | Append `<MarkStepComplete tab="library">` |
| Modify | `src/views/assembling/DedupReview.tsx` | Append `<MarkStepComplete tab="dedup">` |
| Modify | `src/views/arranging/PrismaPanel.tsx` | Append `<MarkStepComplete tab="prisma">` |
| Modify | `src/views/assessing/PdfLibraryPanel.tsx` | Append `<MarkStepComplete tab="library">` |
| Modify | `src/views/assessing/AskCorpusPanel.tsx` | Append `<MarkStepComplete tab="ask">` |
| Modify | `src/views/assessing/ExportPanel.tsx` | Append `<MarkStepComplete tab="export">` |

Settings key convention: `progress.<stage>.<tab>.complete` → `boolean`.

---

## Task 1: Extend `AssemblingState` and add progress queries (Rust)

**Files:**
- Modify: `src-tauri/src/db.rs`

- [ ] **Step 1: Extend the `AssemblingState` struct**

In `src-tauri/src/db.rs`, find the `AssemblingState` struct (around line 86):

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssemblingState {
    pub total_records: i64,
    pub unique_records: i64,
    pub duplicate_records: i64,
    pub import_batches: i64,
}
```

Replace with:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssemblingState {
    pub total_records: i64,
    pub unique_records: i64,
    pub duplicate_records: i64,
    pub import_batches: i64,
    /// True when the project has both a non-empty boolean_query AND
    /// at least one selected database. Drives the Acquisition tab status.
    pub has_acquisition_saved: bool,
    /// True when at least one duplicate cluster is awaiting review
    /// (article rows in status='retrieved' that share a duplicate group with another retrieved article).
    pub has_unresolved_clusters: bool,
}
```

- [ ] **Step 2: Update the `assembling_state` query body**

Find `pub async fn assembling_state(&self) -> Result<AssemblingState>` (around line 599). Replace its body with:

```rust
    pub async fn assembling_state(&self) -> Result<AssemblingState> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM article")
            .fetch_one(&self.pool)
            .await?;
        let unique: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM article WHERE status = 'retrieved'")
                .fetch_one(&self.pool)
                .await?;
        let duplicates: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'excluded' AND exclusion_criterion = 1",
        )
        .fetch_one(&self.pool)
        .await?;
        let batches: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM import_batch")
            .fetch_one(&self.pool)
            .await?;

        let project_row = sqlx::query!(
            "SELECT boolean_query, databases FROM project LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;
        let has_acquisition_saved = project_row
            .as_ref()
            .map(|r| {
                let bq_set = r.boolean_query.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);
                let dbs_set = r.databases.as_deref().map(|s| s != "[]" && !s.trim().is_empty()).unwrap_or(false);
                bq_set && dbs_set
            })
            .unwrap_or(false);

        // Unresolved clusters: articles still in 'retrieved' that share a duplicate_group_id with another 'retrieved' row.
        let unresolved: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article a \
             WHERE a.status = 'retrieved' AND a.duplicate_group_id IS NOT NULL \
             AND EXISTS (SELECT 1 FROM article b WHERE b.status = 'retrieved' \
                         AND b.duplicate_group_id = a.duplicate_group_id AND b.id <> a.id)",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        Ok(AssemblingState {
            total_records: total,
            unique_records: unique,
            duplicate_records: duplicates,
            import_batches: batches,
            has_acquisition_saved,
            has_unresolved_clusters: unresolved > 0,
        })
    }
```

- [ ] **Step 3: Add `ArrangingProgress` struct + query**

Find the closing `}` of `assembling_state` (the `}` after `Ok(AssemblingState { ... })`). Add this immediately after, before the next `pub async fn`:

```rust
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArrangingProgress {
    pub codes_count: i64,
    pub criteria_count: i64,
    pub articles_retrieved: i64,
    pub articles_screened: i64,
    pub articles_included: i64,
    pub articles_coded: i64,
}

impl Db {
    pub async fn arranging_progress(&self) -> Result<ArrangingProgress> {
        let codes_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM framework_code_definition",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let criteria_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM exclusion_criterion",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let articles_retrieved: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'retrieved'",
        )
        .fetch_one(&self.pool)
        .await?;

        let articles_screened: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status IN ('included', 'excluded') \
             AND COALESCE(exclusion_criterion, 0) != 1",
        )
        .fetch_one(&self.pool)
        .await?;

        let articles_included: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'included'",
        )
        .fetch_one(&self.pool)
        .await?;

        let articles_coded: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT article_id) FROM coding",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        Ok(ArrangingProgress {
            codes_count,
            criteria_count,
            articles_retrieved,
            articles_screened,
            articles_included,
            articles_coded,
        })
    }
```

The `impl Db {` opening is intentional — the `}` from Step 2 closes the prior `impl` block, and we open a new one for clarity. (If you prefer a single `impl` block, place the function inside the existing one and skip the wrapper.)

Actually — to avoid breaking the file structure, **put `arranging_progress` inside the existing `impl Db` block**: find the closing `}` of `assembling_state` and insert the function body (without the `}` and `impl Db {` wrappers) directly there. Use this version:

```rust
    pub async fn arranging_progress(&self) -> Result<ArrangingProgress> {
        let codes_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM framework_code_definition",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let criteria_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM exclusion_criterion",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let articles_retrieved: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'retrieved'",
        )
        .fetch_one(&self.pool)
        .await?;

        let articles_screened: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status IN ('included', 'excluded') \
             AND COALESCE(exclusion_criterion, 0) != 1",
        )
        .fetch_one(&self.pool)
        .await?;

        let articles_included: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'included'",
        )
        .fetch_one(&self.pool)
        .await?;

        let articles_coded: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT article_id) FROM coding",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        Ok(ArrangingProgress {
            codes_count,
            criteria_count,
            articles_retrieved,
            articles_screened,
            articles_included,
            articles_coded,
        })
    }
```

The `ArrangingProgress` struct itself goes outside the `impl Db` block, alongside `AssemblingState` (around line 86 — directly after `AssemblingState`).

- [ ] **Step 4: Add `AssessingProgress` struct + query**

Add this struct alongside `ArrangingProgress` (outside the `impl Db` block):

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssessingProgress {
    pub articles_included: i64,
    pub articles_with_pdf: i64,
    pub has_draft: bool,
}
```

Add this method inside the `impl Db` block alongside `arranging_progress`:

```rust
    pub async fn assessing_progress(&self) -> Result<AssessingProgress> {
        let articles_included: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'included'",
        )
        .fetch_one(&self.pool)
        .await?;

        let articles_with_pdf: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT a.id) FROM article a \
             INNER JOIN pdf p ON p.article_id = a.id \
             WHERE a.status = 'included'",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        // Draft persistence: a non-empty value at settings key 'draft.text'.
        let draft_text: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'draft.text'",
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);
        let has_draft = draft_text
            .as_deref()
            .map(|s| !s.trim().is_empty() && s.trim() != "\"\"")
            .unwrap_or(false);

        Ok(AssessingProgress {
            articles_included,
            articles_with_pdf,
            has_draft,
        })
    }
```

- [ ] **Step 5: Verify Rust compiles**

```bash
cd src-tauri && cargo check --message-format=short 2>&1 | tail -10
```

Expected: no errors.

If `query!` macro fails because the `project` table column types don't match what's expected (sqlx offline cache), use `sqlx::query` (without `!`) and `.fetch_optional(...)` with manual row mapping. The function-level fallback is:

```rust
        let row = sqlx::query("SELECT boolean_query, databases FROM project LIMIT 1")
            .fetch_optional(&self.pool)
            .await?;
        let has_acquisition_saved = match row {
            Some(r) => {
                let bq: Option<String> = r.try_get("boolean_query").ok();
                let dbs: Option<String> = r.try_get("databases").ok();
                let bq_set = bq.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);
                let dbs_set = dbs.as_deref().map(|s| s != "[]" && !s.trim().is_empty()).unwrap_or(false);
                bq_set && dbs_set
            }
            None => false,
        };
```

(Add `use sqlx::Row;` near the top of `db.rs` if not already present.)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): progress signals for assembling/arranging/assessing"
```

---

## Task 2: Wire `arranging_progress` and `assessing_progress` Tauri commands

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the two new imports**

Find the existing `use db::{ ... };` line (around line 22) in `src-tauri/src/lib.rs`:

```rust
use db::{
    ArticleRow, AssemblingState, CodingRow, DecisionInput, ExclusionCriterionRow,
    ...
};
```

Add `ArrangingProgress` and `AssessingProgress` to the import list. Inserting them in alphabetical order:

```rust
use db::{
    ArrangingProgress, ArticleRow, AssemblingState, AssessingProgress, CodingRow,
    DecisionInput, ExclusionCriterionRow,
    ...
};
```

- [ ] **Step 2: Add the two Tauri commands**

Find the existing `get_assembling_state` Tauri command (around line 402):

```rust
async fn get_assembling_state(state: State<'_, AppState>) -> Result<AssemblingState, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.assembling_state().await.map_err(err)
}
```

Insert these two commands immediately after it:

```rust
#[tauri::command]
async fn arranging_progress(state: State<'_, AppState>) -> Result<ArrangingProgress, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.arranging_progress().await.map_err(err)
}

#[tauri::command]
async fn assessing_progress(state: State<'_, AppState>) -> Result<AssessingProgress, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.assessing_progress().await.map_err(err)
}
```

- [ ] **Step 3: Register them in `tauri::generate_handler!`**

Find the line `get_assembling_state,` in the `tauri::generate_handler!` block (around line 3084). Add the two new commands immediately after it:

```rust
            get_assembling_state,
            arranging_progress,
            assessing_progress,
```

- [ ] **Step 4: Verify compilation**

```bash
cd src-tauri && cargo check --message-format=short 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(backend): register arranging_progress, assessing_progress commands"
```

---

## Task 3: TypeScript API surface

**Files:**
- Modify: `src/lib/api.ts`

- [ ] **Step 1: Extend `AssemblingState` and add the two new types**

In `src/lib/api.ts`, find the existing `interface AssemblingState`:

```ts
export interface AssemblingState {
  total_records: number;
  unique_records: number;
  duplicate_records: number;
  import_batches: number;
}
```

Replace with:

```ts
export interface AssemblingState {
  total_records: number;
  unique_records: number;
  duplicate_records: number;
  import_batches: number;
  has_acquisition_saved: boolean;
  has_unresolved_clusters: boolean;
}

export interface ArrangingProgress {
  codes_count: number;
  criteria_count: number;
  articles_retrieved: number;
  articles_screened: number;
  articles_included: number;
  articles_coded: number;
}

export interface AssessingProgress {
  articles_included: number;
  articles_with_pdf: number;
  has_draft: boolean;
}
```

- [ ] **Step 2: Add the two API methods**

Find the existing `assemblingState()` method on the `api` object. Add these two methods directly after it:

```ts
  arrangingProgress() {
    return invoke<ArrangingProgress>("arranging_progress");
  },
  assessingProgress() {
    return invoke<AssessingProgress>("assessing_progress");
  },
```

- [ ] **Step 3: Verify**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): arrangingProgress, assessingProgress + AssemblingState fields"
```

---

## Task 4: `<ProgressionTabStrip>` component

**Files:**
- Create: `src/components/ProgressionTabStrip.tsx`

- [ ] **Step 1: Create the file**

Write the following content to `src/components/ProgressionTabStrip.tsx`:

```tsx
import { motion } from "framer-motion";
import { cn } from "../lib/utils";

export type TabStatus = "done" | "active" | "todo";

export interface ProgressionTab<Id extends string> {
  id: Id;
  label: string;
  icon: React.ComponentType<{ size?: number; strokeWidth?: number; className?: string }>;
  status: TabStatus;
  number: number; // 1-based
}

interface Props<Id extends string> {
  tabs: ProgressionTab<Id>[];
  onSelect: (id: Id) => void;
  underlineLayoutId: string;
}

export function ProgressionTabStrip<Id extends string>({ tabs, onSelect, underlineLayoutId }: Props<Id>) {
  return (
    <div className="flex items-end gap-1 border-b border-[var(--color-paper-300)]">
      {tabs.map((t) => {
        const Icon = t.icon;
        return (
          <button
            key={t.id}
            onClick={() => onSelect(t.id)}
            className={cn(
              "no-drag relative px-5 pt-3 pb-3.5 text-[13px] font-medium",
              "transition-paper flex items-center gap-2",
              t.status === "active"
                ? "text-[var(--color-ink-900)]"
                : t.status === "done"
                ? "text-[var(--color-paper-700)] hover:text-[var(--color-ink-700)]"
                : "text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)]",
            )}
          >
            <Badge status={t.status} number={t.number} />
            <Icon size={14} strokeWidth={1.75} />
            {t.label}
            {t.status === "active" && (
              <motion.div
                layoutId={underlineLayoutId}
                className="absolute left-0 right-0 -bottom-px h-0.5 bg-[var(--color-ink-700)]"
                transition={{ type: "spring", stiffness: 500, damping: 35 }}
              />
            )}
          </button>
        );
      })}
    </div>
  );
}

function Badge({ status, number }: { status: TabStatus; number: number }) {
  if (status === "done") {
    return (
      <span
        className="inline-flex items-center justify-center w-[18px] h-[18px] rounded-full bg-[var(--color-accent-500)] border border-[var(--color-accent-500)]"
        aria-label="Completed"
      >
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <path
            d="M1.5 5L4 7.5L8.5 2.5"
            stroke="white"
            strokeWidth="1.6"
            fill="none"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </span>
    );
  }
  if (status === "active") {
    return (
      <span
        className="inline-flex items-center justify-center w-[18px] h-[18px] rounded-full bg-[var(--color-ink-900)] text-white font-mono text-[10px] font-bold"
        aria-label={`Step ${number} (current)`}
      >
        {number}
      </span>
    );
  }
  return (
    <span
      className="inline-flex items-center justify-center w-[18px] h-[18px] rounded-full bg-[var(--color-accent-50)] border border-[var(--color-accent-300)] text-[var(--color-paper-500)] font-mono text-[10px] font-bold"
      aria-label={`Step ${number}`}
    >
      {number}
    </span>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/ProgressionTabStrip.tsx
git commit -m "feat(ui): ProgressionTabStrip with numbered + check status badges"
```

---

## Task 5: `<MarkStepComplete>` component

**Files:**
- Create: `src/components/MarkStepComplete.tsx`

- [ ] **Step 1: Create the file**

Write the following content to `src/components/MarkStepComplete.tsx`:

```tsx
import { useState } from "react";
import { Check } from "lucide-react";
import { api } from "../lib/api";
import { cn } from "../lib/utils";

interface Props {
  stage: "assembling" | "arranging" | "assessing";
  tab: string;
  isComplete: boolean;
  onChanged: () => void;
}

export function MarkStepComplete({ stage, tab, isComplete, onChanged }: Props) {
  const [busy, setBusy] = useState(false);
  const key = `progress.${stage}.${tab}.complete`;

  const setFlag = async (value: boolean) => {
    setBusy(true);
    try {
      await api.setSetting(key, value);
      onChanged();
    } finally {
      setBusy(false);
    }
  };

  if (isComplete) {
    return (
      <div className="mt-8 pt-4 border-t border-[var(--color-paper-200)] flex items-center gap-3 text-[12px]">
        <span className="inline-flex items-center gap-1.5 text-[var(--color-ok-600)]">
          <Check size={13} strokeWidth={2} />
          Marked complete
        </span>
        <button
          type="button"
          onClick={() => setFlag(false)}
          disabled={busy}
          className={cn(
            "text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] underline-offset-2 hover:underline transition-paper",
            busy && "opacity-50 cursor-wait",
          )}
        >
          Undo
        </button>
      </div>
    );
  }

  return (
    <div className="mt-8 pt-4 border-t border-[var(--color-paper-200)]">
      <button
        type="button"
        onClick={() => setFlag(true)}
        disabled={busy}
        className={cn(
          "text-[12px] text-[var(--color-ink-700)] hover:text-[var(--color-accent-600)] underline-offset-2 hover:underline transition-paper",
          busy && "opacity-50 cursor-wait",
        )}
      >
        Mark this step complete →
      </button>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/MarkStepComplete.tsx
git commit -m "feat(ui): MarkStepComplete inline footer link"
```

---

## Task 6: `useTabProgress` hook

**Files:**
- Create: `src/lib/useTabProgress.ts`

- [ ] **Step 1: Create the file**

Write the following content to `src/lib/useTabProgress.ts`:

```ts
import type { TabStatus } from "../components/ProgressionTabStrip";

/**
 * Compute status for each tab.
 *   - active: tab.id === currentTab
 *   - done: dataPredicate returns true OR manual flag set
 *   - todo: otherwise
 *
 * The active tab takes precedence — even if the user has marked it complete
 * or its data predicate returns true, while they're viewing it the status
 * shows as 'active' so the underline + numbered badge remain consistent.
 */
export function useTabProgress<Id extends string>(
  currentTab: Id,
  predicates: Record<Id, () => boolean>,
  manualFlags: Record<Id, boolean>,
): Record<Id, TabStatus> {
  const result = {} as Record<Id, TabStatus>;
  for (const id in predicates) {
    if (id === currentTab) {
      result[id] = "active";
    } else {
      let done = false;
      try {
        done = predicates[id]() || !!manualFlags[id];
      } catch {
        done = !!manualFlags[id];
      }
      result[id] = done ? "done" : "todo";
    }
  }
  return result;
}

/** Read progress flags for one stage from the loaded settings dict. */
export function readManualFlags<Id extends string>(
  stage: "assembling" | "arranging" | "assessing",
  ids: Id[],
  settings: Record<string, unknown>,
): Record<Id, boolean> {
  const flags = {} as Record<Id, boolean>;
  for (const id of ids) {
    const key = `progress.${stage}.${id}.complete`;
    flags[id] = settings[key] === true;
  }
  return flags;
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/useTabProgress.ts
git commit -m "feat(ui): useTabProgress hook + readManualFlags helper"
```

---

## Task 7: Wire AssemblingStage

**Files:**
- Modify: `src/views/assembling/AssemblingStage.tsx`
- Modify: `src/views/assembling/JournalScopePanel.tsx`
- Modify: `src/views/assembling/ArticleLibrary.tsx`
- Modify: `src/views/assembling/DedupReview.tsx`

- [ ] **Step 1: Update imports in `AssemblingStage.tsx`**

Find the existing imports block at the top of `src/views/assembling/AssemblingStage.tsx`. Add these lines after the existing `import` statements:

```tsx
import { ProgressionTabStrip, type ProgressionTab } from "../../components/ProgressionTabStrip";
import { useTabProgress, readManualFlags } from "../../lib/useTabProgress";
```

- [ ] **Step 2: Load settings alongside assembling state**

Find:

```tsx
  const [assembling, setAssembling] = useState<AssemblingState | null>(null);
  const [refreshTick, setRefreshTick] = useState(0);

  useEffect(() => {
    api.assemblingState().then(setAssembling).catch(() => {});
  }, [refreshTick]);
```

Replace with:

```tsx
  const [assembling, setAssembling] = useState<AssemblingState | null>(null);
  const [settings, setSettings] = useState<Record<string, unknown>>({});
  const [refreshTick, setRefreshTick] = useState(0);

  useEffect(() => {
    api.assemblingState().then(setAssembling).catch(() => {});
    api.getSettings().then(setSettings).catch(() => {});
  }, [refreshTick]);
```

- [ ] **Step 3: Compute the per-tab status map**

Find `const onChanged = () => setRefreshTick((t) => t + 1);` (around line 123) and add **immediately after it**:

```tsx
  const TAB_IDS: Tab[] = ["acquisition", "import", "journals", "library", "dedup", "checkpoint"];
  const manualFlags = readManualFlags("assembling", TAB_IDS, settings);
  const status = useTabProgress<Tab>(tab, {
    acquisition: () => !!assembling?.has_acquisition_saved,
    import: () => (assembling?.import_batches ?? 0) >= 1,
    journals: () => false, // manual only
    library: () => false, // manual only
    dedup: () => assembling != null && !assembling.has_unresolved_clusters && (assembling.duplicate_records ?? 0) > 0,
    checkpoint: () => project.current_stage !== "assembling",
  }, manualFlags);
```

(Note: the `dedup` predicate requires that some duplicates were detected AND none are unresolved — otherwise an empty corpus would show dedup as done before any work was started.)

- [ ] **Step 4: Build the typed tab list**

Immediately after the status map, add:

```tsx
  const numberedTabs: ProgressionTab<Tab>[] = tabs.map((t, i) => ({
    id: t.id,
    label: t.label,
    icon: t.icon,
    status: status[t.id],
    number: i + 1,
  }));
```

- [ ] **Step 5: Replace the inline `<nav>` block with `<ProgressionTabStrip>`**

Find the entire `<nav className="px-6 xl:px-12 mb-6">` block (around lines 160–190) and replace it with:

```tsx
      <nav className="px-6 xl:px-12 mb-6">
        <ProgressionTabStrip
          tabs={numberedTabs}
          onSelect={setTab}
          underlineLayoutId="assembling-tab-underline"
        />
      </nav>
```

- [ ] **Step 6: Wire `<MarkStepComplete>` into JournalScopePanel**

In `src/views/assembling/JournalScopePanel.tsx`, add at the top:

```tsx
import { MarkStepComplete } from "../../components/MarkStepComplete";
```

Find the component's `Props` interface. Add an optional pair:

```tsx
interface Props {
  // ... existing fields ...
  manualComplete?: boolean;
  onChanged?: () => void;
}
```

(If the component already has an `onChanged` prop, don't redeclare — just add `manualComplete`.)

At the very end of the panel's main JSX return, just before the outermost closing tag, add:

```tsx
      <MarkStepComplete
        stage="assembling"
        tab="journals"
        isComplete={!!manualComplete}
        onChanged={onChanged ?? (() => {})}
      />
```

- [ ] **Step 7: Pass the props from `AssemblingStage` to `JournalScopePanel`**

Find `{tab === "journals" && <JournalScopePanel />}` (around line 213) and replace with:

```tsx
              {tab === "journals" && (
                <JournalScopePanel
                  manualComplete={settings["progress.assembling.journals.complete"] === true}
                  onChanged={onChanged}
                />
              )}
```

- [ ] **Step 8: Repeat the same wiring for `ArticleLibrary` and `DedupReview`**

For `ArticleLibrary` (`src/views/assembling/ArticleLibrary.tsx`):
- Add the import: `import { MarkStepComplete } from "../../components/MarkStepComplete";`
- Add `manualComplete?: boolean` to its Props.
- Append to its JSX:
  ```tsx
        <MarkStepComplete
          stage="assembling"
          tab="library"
          isComplete={!!manualComplete}
          onChanged={onChanged}
        />
  ```
- In `AssemblingStage.tsx`, update the call:
  ```tsx
                {tab === "library" && (
                  <ArticleLibrary
                    refreshKey={refreshTick}
                    onChanged={onChanged}
                    manualComplete={settings["progress.assembling.library.complete"] === true}
                  />
                )}
  ```

For `DedupReview` (`src/views/assembling/DedupReview.tsx`): same three changes, with `tab="dedup"` and `progress.assembling.dedup.complete`.

- [ ] **Step 9: Verify TypeScript compiles**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 10: Commit**

```bash
git add src/views/assembling/
git commit -m "feat(assembling): progression tabs + Mark complete on optional panels"
```

---

## Task 8: Wire ArrangingStage

**Files:**
- Modify: `src/views/arranging/ArrangingStage.tsx`
- Modify: `src/views/arranging/PrismaPanel.tsx`

- [ ] **Step 1: Update imports in `ArrangingStage.tsx`**

Add at the top of the file:

```tsx
import { ProgressionTabStrip, type ProgressionTab } from "../../components/ProgressionTabStrip";
import { useTabProgress, readManualFlags } from "../../lib/useTabProgress";
import type { ArrangingProgress } from "../../lib/api";
```

- [ ] **Step 2: Load progress data**

Find the existing state declarations (around lines 81–90 — `const [tab, setTab]`, `refreshTick`, etc.). Add these lines after them:

```tsx
  const [progress, setProgress] = useState<ArrangingProgress | null>(null);
  const [settings, setSettings] = useState<Record<string, unknown>>({});

  useEffect(() => {
    api.arrangingProgress().then(setProgress).catch(() => {});
    api.getSettings().then(setSettings).catch(() => {});
  }, [project.id, refreshTick]);
```

- [ ] **Step 3: Compute status and build numbered tabs**

Find `const onChanged = async () => { ... };` (around line 96). Add **immediately after** the closing `};`:

```tsx
  const TAB_IDS: Tab[] = ["organization", "criteria", "screening", "coding", "prisma"];
  const manualFlags = readManualFlags("arranging", TAB_IDS, settings);
  const status = useTabProgress<Tab>(tab, {
    organization: () => (progress?.codes_count ?? 0) >= 1,
    criteria: () => (progress?.criteria_count ?? 0) >= 1,
    screening: () => progress != null && progress.articles_retrieved > 0 && progress.articles_screened >= progress.articles_retrieved,
    coding: () => progress != null && progress.articles_included > 0 && progress.articles_coded >= progress.articles_included,
    prisma: () => false, // manual only
  }, manualFlags);

  const numberedTabs: ProgressionTab<Tab>[] = tabs.map((t, i) => ({
    id: t.id,
    label: t.label,
    icon: t.icon,
    status: status[t.id],
    number: i + 1,
  }));
```

- [ ] **Step 4: Replace the `<nav>` block**

Find the existing tab strip `<nav>` block (search for `arranging-tab-underline`) and replace its body with:

```tsx
        <ProgressionTabStrip
          tabs={numberedTabs}
          onSelect={setTab}
          underlineLayoutId="arranging-tab-underline"
        />
```

- [ ] **Step 5: Add `<MarkStepComplete>` to `PrismaPanel`**

In `src/views/arranging/PrismaPanel.tsx`:
- Add import: `import { MarkStepComplete } from "../../components/MarkStepComplete";`
- Add `manualComplete?: boolean; onChanged?: () => void` to its Props.
- Append to the panel's JSX, before the outermost closing tag:

  ```tsx
      <MarkStepComplete
        stage="arranging"
        tab="prisma"
        isComplete={!!manualComplete}
        onChanged={onChanged ?? (() => {})}
      />
  ```

- [ ] **Step 6: Pass the props in `ArrangingStage.tsx`**

Find the line `{tab === "prisma" && <PrismaPanel refreshKey={refreshTick} />}` (around line 199) and replace with:

```tsx
              {tab === "prisma" && (
                <PrismaPanel
                  refreshKey={refreshTick}
                  manualComplete={settings["progress.arranging.prisma.complete"] === true}
                  onChanged={onChanged}
                />
              )}
```

- [ ] **Step 7: Verify and commit**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

```bash
git add src/views/arranging/
git commit -m "feat(arranging): progression tabs + Mark complete on PRISMA"
```

---

## Task 9: Wire AssessingStage

**Files:**
- Modify: `src/views/assessing/AssessingStage.tsx`
- Modify: `src/views/assessing/PdfLibraryPanel.tsx`
- Modify: `src/views/assessing/AskCorpusPanel.tsx`
- Modify: `src/views/assessing/ExportPanel.tsx`

- [ ] **Step 1: Update imports in `AssessingStage.tsx`**

Add at the top of the file:

```tsx
import { ProgressionTabStrip, type ProgressionTab } from "../../components/ProgressionTabStrip";
import { useTabProgress, readManualFlags } from "../../lib/useTabProgress";
import type { AssessingProgress } from "../../lib/api";
```

- [ ] **Step 2: Load progress data**

Find the existing state declarations (around lines 58–60). Add these lines after them:

```tsx
  const [progress, setProgress] = useState<AssessingProgress | null>(null);
  const [settings, setSettings] = useState<Record<string, unknown>>({});

  useEffect(() => {
    api.assessingProgress().then(setProgress).catch(() => {});
    api.getSettings().then(setSettings).catch(() => {});
  }, [project.id, refreshTick]);
```

- [ ] **Step 3: Compute status and build numbered tabs**

Find `const onChanged = () => setRefreshTick((t) => t + 1);` (around line 60) and add **immediately after**:

```tsx
  const TAB_IDS: Tab[] = ["library", "ask", "draft", "export"];
  const manualFlags = readManualFlags("assessing", TAB_IDS, settings);
  const status = useTabProgress<Tab>(tab, {
    library: () =>
      progress != null && progress.articles_included > 0 &&
      progress.articles_with_pdf / progress.articles_included >= 0.8,
    ask: () => false, // manual only
    draft: () => !!progress?.has_draft,
    export: () => false, // manual only (terminal step)
  }, manualFlags);

  const numberedTabs: ProgressionTab<Tab>[] = tabs.map((t, i) => ({
    id: t.id,
    label: t.label,
    icon: t.icon,
    status: status[t.id],
    number: i + 1,
  }));
```

- [ ] **Step 4: Replace the `<nav>` block**

Find the tab `<nav>` (search for `assessing-tab-underline`) and replace its body with:

```tsx
        <ProgressionTabStrip
          tabs={numberedTabs}
          onSelect={setTab}
          underlineLayoutId="assessing-tab-underline"
        />
```

- [ ] **Step 5: Add `<MarkStepComplete>` to `PdfLibraryPanel`, `AskCorpusPanel`, `ExportPanel`**

For each of the three panels, do the same three things:

**`PdfLibraryPanel.tsx`:**
- Add import: `import { MarkStepComplete } from "../../components/MarkStepComplete";`
- Add `manualComplete?: boolean` to Props (component already has `onChanged`).
- Append to the JSX:
  ```tsx
        <MarkStepComplete
          stage="assessing"
          tab="library"
          isComplete={!!manualComplete}
          onChanged={onChanged}
        />
  ```

**`AskCorpusPanel.tsx`:**
- Same three changes, with `tab="ask"` and `progress.assessing.ask.complete`. The panel may not currently take an `onChanged` prop — if so, also add `onChanged?: () => void` to its Props and use `onChanged ?? (() => {})` in the call.

**`ExportPanel.tsx`:**
- Same three changes, with `tab="export"`.

- [ ] **Step 6: Pass `manualComplete` from `AssessingStage`**

In `AssessingStage.tsx`, update the panel callsites:

```tsx
              {tab === "library" && (
                <PdfLibraryPanel
                  refreshKey={refreshTick}
                  onChanged={onChanged}
                  manualComplete={settings["progress.assessing.library.complete"] === true}
                />
              )}
              {tab === "ask" && (
                <AskCorpusPanel
                  manualComplete={settings["progress.assessing.ask.complete"] === true}
                  onChanged={onChanged}
                />
              )}
              {tab === "draft" && <DraftPanel project={project} />}
              {tab === "export" && (
                <ExportPanel
                  manualComplete={settings["progress.assessing.export.complete"] === true}
                  onChanged={onChanged}
                />
              )}
```

(Draft has no manual button — it has a real data predicate.)

- [ ] **Step 7: Verify and commit**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

```bash
git add src/views/assessing/
git commit -m "feat(assessing): progression tabs + Mark complete on viewer panels"
```

---

## Task 10: Manual smoke test

**Prerequisites:**
- Tauri dev server running (`npm run tauri dev`)
- A project with at least some content in each stage (or willing to walk through fresh)

- [ ] **Step 1: Open a fresh project to onboarding completion**

Open a project that has just left onboarding. Enter Stage 1 (Assembling).

- [ ] **Step 2: Verify Assembling tabs**

Confirm:
- Six numbered tabs in the order: 1 Acquisition, 2 Importing, 3 Journals, 4 Library, 5 Deduplication, 6 Checkpoint.
- Active tab (Acquisition) shows the solid black `1` badge.
- All other tabs show light `2`–`6` badges with no check.

- [ ] **Step 3: Verify the Acquisition data predicate**

- Pick at least one database checkbox.
- Type any non-empty boolean query.
- Click **Save acquisition decisions**.
- Click another tab to deselect Acquisition. Acquisition's badge should now show a green check (`done` state).

- [ ] **Step 4: Verify the Import data predicate**

- Go to the Importing tab. Import a small file (a sample RIS or BibTeX export).
- Click another tab. Importing's badge should show a check.

- [ ] **Step 5: Verify the Journals manual flag**

- Click the Journals tab.
- Scroll to the bottom of the panel. Click **Mark this step complete →**.
- Confirm "Marked complete" + Undo link appear.
- Click another tab. Journals shows a check.
- Reload the project (close and reopen). Journals still shows a check.
- Click Undo. Confirm the check disappears.

- [ ] **Step 6: Stage 2 — Arranging**

Navigate to Stage 2. Confirm:
- Five numbered tabs: 1 Organisation, 2 Purification, 3 Screening, 4 Coding, 5 PRISMA flow.
- Add a code in Organisation; confirm its badge gets a check after switching tabs.
- Add a criterion in Purification; confirm its badge gets a check.
- Mark PRISMA complete via the manual link; confirm the check.

- [ ] **Step 7: Stage 3 — Assessing**

Navigate to Stage 3. Confirm:
- Four numbered tabs: 1 PDF library, 2 Ask the corpus, 3 Draft, 4 Export.
- The PDF library shows todo unless 80%+ of included articles have a PDF (or the manual flag is set).
- Save the draft; Draft tab gets a check.
- Mark Export complete via the manual link.

- [ ] **Step 8: Tab strip layout under narrow widths**

Resize the window to ~900px wide. Confirm:
- Badges don't break the tab strip layout (still on one row, no wrap).
- Active underline remains aligned.

- [ ] **Step 9: Final commit (if any polish needed)**

If the smoke test surfaces a styling tweak or copy fix, make the change and commit:

```bash
git add src/...
git commit -m "polish(progression): smoke-test fixes"
```

If no fix needed, this task is just the smoke test pass — no commit.

---

## Self-Review

**Spec coverage:**
- ✅ `<ProgressionTabStrip>` with done/active/todo styles — Task 4
- ✅ `useTabProgress` hook — Task 6
- ✅ `<MarkStepComplete>` text link — Task 5
- ✅ Per-tab predicates per stage — Tasks 7–9 (data signals from Tasks 1–3)
- ✅ Manual flag persistence via existing `set_setting` — Task 5 (component) + Task 7–9 (callsites)
- ✅ All three stages wired — Tasks 7, 8, 9
- ✅ Active tab takes precedence over done — Task 6 (hook order)
- ✅ Predicate failures default to todo (not crash) — Task 6 (try/catch)
- ✅ Existing Framer Motion underline preserved — Task 4 (`underlineLayoutId` prop)

**Type consistency:**
- `TabStatus = "done" | "active" | "todo"` exported from `ProgressionTabStrip.tsx` (Task 4) — used in `useTabProgress.ts` (Task 6) and the three stage views.
- `ProgressionTab<Id>` shape matches the per-stage `numberedTabs` construction in Tasks 7–9.
- `ArrangingProgress`, `AssessingProgress` field names match between Rust (Task 1) and TS (Task 3).
- The settings key convention `progress.<stage>.<tab>.complete` is consistent: written by `<MarkStepComplete>` (Task 5), read by `readManualFlags` (Task 6), passed in via `manualComplete` prop (Tasks 7–9).

**Placeholder scan:**
No "TBD" / "TODO" / vague stand-ins. Each step shows the literal code or command. The only "fallback" content (Step 5 of Task 1) is documented for the case where sqlx's compile-time check rejects the `query!` macro, with the alternative shown.
