# Boolean Query AI Suggestion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an AI "Suggest" / "Advise" button to the boolean query textarea in Stage 1 · Assembling that opens a modal with a generated or improved query the user can accept.

**Architecture:** A new `suggest_query` Rust function in `suggest.rs` builds a prompt from project context and calls LM Studio. A new Tauri command `suggest_boolean_query` wires it into the app state. The React side adds a `QuerySuggestionModal` component and wires the button + Ctrl+Z undo into `AcquisitionForm`.

**Tech Stack:** Rust/Tauri (backend), React/TypeScript (frontend), Tailwind CSS + CSS variables (styling), Framer Motion (animation), Lucide icons, LM Studio local inference

---

## File Map

| Action | Path | Purpose |
|--------|------|---------|
| Modify | `src-tauri/src/suggest.rs` | Add `SuggestQueryResult` struct and `suggest_query` function |
| Modify | `src-tauri/src/lib.rs` | Add `SuggestQueryArgs` struct, `suggest_boolean_query` command, register it |
| Modify | `src/lib/api.ts` | Add `SuggestQueryResult` interface and `suggestBooleanQuery` method |
| Create | `src/components/QuerySuggestionModal.tsx` | Modal with loading state, query display, rationale, Accept/Dismiss |
| Modify | `src/views/assembling/AcquisitionForm.tsx` | Button, modal state, `previousQuery` ref, Ctrl+Z handler |

---

## Task 1: Add `suggest_query` to `suggest.rs`

**Files:**
- Modify: `src-tauri/src/suggest.rs`

- [ ] **Step 1: Add `SuggestQueryResult` struct at the top of `suggest.rs`, after the existing `SuggestResult` struct (around line 27)**

Add this block after the closing `}` of `SuggestResult`:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SuggestQueryResult {
    pub query: String,
    pub rationale: String,
    pub model: Option<String>,
    pub latency_ms: u64,
}
```

- [ ] **Step 2: Add `suggest_query` function at the bottom of `suggest.rs`, after `parse_suggestions`**

```rust
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

    let result = lmstudio::chat(base_url, model, &messages, Some(0.3)).await?;

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
    let slice = &s[start..end];
    let v: serde_json::Value = serde_json::from_str(slice).ok()?;
    let query = v.get("query")?.as_str()?.to_string();
    let rationale = v.get("rationale")?.as_str()?.to_string();
    Some((query, rationale))
}
```

- [ ] **Step 3: Verify the Rust code compiles**

```bash
cd src-tauri && cargo check 2>&1 | tail -20
```

Expected: no errors (warnings about unused imports are fine).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/suggest.rs
git commit -m "feat(backend): add suggest_query for boolean query AI suggestion"
```

---

## Task 2: Add `suggest_boolean_query` Tauri command to `lib.rs`

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the import for `SuggestQueryResult` at the top of `lib.rs`**

Find the existing import line (around line 23):
```rust
use lmstudio::{ChatMessage, ChatResult, LmModel};
```

Add `SuggestQueryResult` to the suggest import. Find the existing suggest usage and add after the existing imports block a use for the new type — actually `SuggestQueryResult` is accessed as `suggest::SuggestQueryResult` (already namespaced), so no import change is needed.

- [ ] **Step 2: Add the args struct and command after the `suggest_codes` command (around line 684)**

Find the closing `}` of `suggest_codes` (after `Ok(result)`). Insert the following block immediately after it:

```rust
#[derive(Debug, Deserialize)]
struct SuggestQueryArgs {
    databases: Vec<String>,
    existing_query: Option<String>,
}

#[tauri::command]
async fn suggest_boolean_query(
    args: SuggestQueryArgs,
    state: State<'_, AppState>,
) -> Result<suggest::SuggestQueryResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let project = db.get_project().await.map_err(err)?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings
        .get("lmstudio.base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:1234/v1")
        .to_string();
    let model = settings
        .get("lmstudio.chat_model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if model.is_empty() {
        return Err("Pick a chat model in Settings first.".into());
    }
    let result = suggest::suggest_query(
        &base_url,
        &model,
        project.domain.as_deref().unwrap_or(""),
        project.framework.as_deref().unwrap_or(""),
        project.rq_what.as_deref(),
        project.rq_how.as_deref(),
        project.rq_where.as_deref(),
        &args.databases,
        args.existing_query.as_deref(),
    )
    .await
    .map_err(err)?;
    Ok(result)
}
```

- [ ] **Step 3: Register `suggest_boolean_query` in `invoke_handler!`**

Find the line `suggest_codes,` in the `tauri::generate_handler!` block (around line 1120). Add the new command on the line immediately after it:

```rust
            suggest_codes,
            suggest_boolean_query,
```

- [ ] **Step 4: Verify the Rust code compiles**

```bash
cd src-tauri && cargo check 2>&1 | tail -20
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(backend): register suggest_boolean_query Tauri command"
```

---

## Task 3: Add `suggestBooleanQuery` to `api.ts`

**Files:**
- Modify: `src/lib/api.ts`

- [ ] **Step 1: Add `SuggestQueryResult` interface**

Find the existing `SuggestResult` interface (around line 185). Add the new interface immediately after its closing `}`:

```ts
export interface SuggestQueryResult {
  query: string;
  rationale: string;
  model: string | null;
  latency_ms: number;
}
```

- [ ] **Step 2: Add `suggestBooleanQuery` method to the `api` object**

Find the `suggestCodes` method (around line 372):
```ts
  suggestCodes(article_id: number, dimension: string) {
    return invoke<SuggestResult>("suggest_codes", { args: { article_id, dimension } });
  },
```

Add the new method immediately after it:

```ts
  suggestBooleanQuery(databases: string[], existingQuery: string | null) {
    return invoke<SuggestQueryResult>("suggest_boolean_query", {
      args: { databases, existing_query: existingQuery },
    });
  },
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
npx tsc --noEmit 2>&1 | head -30
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): add suggestBooleanQuery typed method"
```

---

## Task 4: Create `QuerySuggestionModal.tsx`

**Files:**
- Create: `src/components/QuerySuggestionModal.tsx`

- [ ] **Step 1: Create the file**

```tsx
import { useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Loader2, Sparkles, X } from "lucide-react";
import { Button } from "./ui/Button";
import { Card } from "./ui/Card";
import type { SuggestQueryResult } from "../lib/api";

interface Props {
  open: boolean;
  mode: "suggest" | "advise";
  result: SuggestQueryResult | null;
  error: string | null;
  onAccept: (query: string) => void;
  onDismiss: () => void;
}

export function QuerySuggestionModal({ open, mode, result, error, onAccept, onDismiss }: Props) {
  // Close on Escape
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onDismiss();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onDismiss]);

  return (
    <AnimatePresence>
      {open && (
        <>
          {/* Backdrop */}
          <motion.div
            key="backdrop"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="fixed inset-0 bg-black/30 z-40"
            onClick={onDismiss}
          />

          {/* Modal */}
          <motion.div
            key="modal"
            initial={{ opacity: 0, scale: 0.96, y: 8 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: 8 }}
            transition={{ duration: 0.2, ease: [0.2, 0.7, 0.2, 1] }}
            className="fixed inset-0 z-50 flex items-center justify-center p-6 pointer-events-none"
          >
            <Card className="pointer-events-auto w-full max-w-2xl p-6 shadow-[var(--shadow-paper)]">
              {/* Header */}
              <div className="flex items-start justify-between mb-4">
                <div className="flex items-center gap-2">
                  <Sparkles size={15} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                  <span className="eyebrow">
                    {mode === "suggest" ? "AI Suggestion" : "AI Advisement"}
                  </span>
                </div>
                <button
                  onClick={onDismiss}
                  className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
                  aria-label="Dismiss"
                >
                  <X size={14} strokeWidth={1.75} />
                </button>
              </div>

              {/* Loading state */}
              {!result && !error && (
                <div className="flex items-center gap-3 py-8 justify-center text-[var(--color-paper-500)]">
                  <Loader2 size={18} className="animate-spin" />
                  <span className="text-[14px]">Thinking…</span>
                </div>
              )}

              {/* Error state */}
              {error && (
                <div className="px-4 py-3 rounded-[var(--radius-sm)] bg-[var(--color-accent-50)] border border-[var(--color-accent-300)] text-[13px] text-[var(--color-accent-700)]">
                  {error}
                </div>
              )}

              {/* Result state */}
              {result && (
                <>
                  <pre className="font-mono text-[12px] leading-relaxed whitespace-pre-wrap bg-[var(--color-paper-100)] border border-[var(--color-paper-200)] rounded-[var(--radius-sm)] px-4 py-3 text-[var(--color-ink-800)] mb-3 max-h-60 overflow-y-auto">
                    {result.query}
                  </pre>
                  {result.rationale && (
                    <p className="text-[12px] text-[var(--color-paper-600)] leading-relaxed mb-4">
                      {result.rationale}
                    </p>
                  )}
                  <div className="flex items-center gap-3">
                    <Button variant="primary" size="sm" onClick={() => onAccept(result.query)}>
                      Accept
                    </Button>
                    <Button variant="ghost" size="sm" onClick={onDismiss}>
                      Dismiss
                    </Button>
                  </div>
                </>
              )}

              {/* Error state dismiss button */}
              {error && (
                <div className="mt-4">
                  <Button variant="ghost" size="sm" onClick={onDismiss}>
                    Dismiss
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
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
npx tsc --noEmit 2>&1 | head -30
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/QuerySuggestionModal.tsx
git commit -m "feat(ui): add QuerySuggestionModal component"
```

---

## Task 5: Wire button, modal, and Ctrl+Z into `AcquisitionForm.tsx`

**Files:**
- Modify: `src/views/assembling/AcquisitionForm.tsx`

- [ ] **Step 1: Add imports at the top of `AcquisitionForm.tsx`**

Find the existing import block. Add `useRef` to the React import and add the new component and API type:

```tsx
import { useState, useRef } from "react";
```
(Replace the existing `import { useState } from "react";` on line 1)

Add after the existing `import { Button }` line:
```tsx
import { QuerySuggestionModal } from "../../components/QuerySuggestionModal";
import type { SuggestQueryResult } from "../../lib/api";
```

- [ ] **Step 2: Add modal state and `previousQuery` ref inside `AcquisitionForm` function**

Find the existing state declarations (around line 26–31):
```tsx
  const [databases, setDatabases] = useState<Set<string>>(new Set());
  const [start, setStart] = useState(project.search_period_start ?? "");
  const [end, setEnd] = useState(project.search_period_end ?? "");
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [savedAt, setSavedAt] = useState<number | null>(null);
```

Add these three lines directly after `savedAt`:
```tsx
  const [modalOpen, setModalOpen] = useState(false);
  const [suggestResult, setSuggestResult] = useState<SuggestQueryResult | null>(null);
  const [suggestError, setSuggestError] = useState<string | null>(null);
  const previousQuery = useRef<string | null>(null);
```

- [ ] **Step 3: Add `handleSuggest` function inside `AcquisitionForm`, after the `save` function (around line 84)**

```tsx
  const handleSuggest = async () => {
    setSuggestResult(null);
    setSuggestError(null);
    setModalOpen(true);
    try {
      const result = await api.suggestBooleanQuery(
        Array.from(databases),
        query.trim() || null,
      );
      setSuggestResult(result);
    } catch (e: any) {
      setSuggestError(
        e?.message?.includes("chat model")
          ? "Pick a chat model in Settings first."
          : e?.message?.includes("LM Studio") || e?.message?.includes("loopback")
          ? "Could not reach LM Studio — check Settings."
          : "The model returned an unexpected response. Try again.",
      );
    }
  };

  const handleAccept = (suggested: string) => {
    previousQuery.current = query;
    setQuery(suggested);
    setModalOpen(false);
    setSuggestResult(null);
  };

  const handleQueryKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "z" && previousQuery.current !== null) {
      e.preventDefault();
      setQuery(previousQuery.current);
      previousQuery.current = null;
    }
  };
```

- [ ] **Step 4: Update the Boolean query section in the JSX**

Find this block (around line 172–188):
```tsx
        {/* Query */}
        <section>
          <SectionHeader
            icon={<Search size={14} strokeWidth={1.75} />}
            title="Boolean query"
            description="The actual string you'll paste into each database's advanced search. You'll run the search yourself in your normal browser."
          />
          <div className="mt-4">
            <TextArea
              label="Query"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              rows={6}
              placeholder={`("social commerce" OR "s-commerce" OR "social shopping")\nAND ("consumer behavi*" OR "buyer behavi*")\nAND ("engagement" OR "purchase" OR "intention")`}
              hint="Use OR between synonyms, AND between concepts. Wildcards (*) where supported."
            />
          </div>
        </section>
```

Replace it with:
```tsx
        {/* Query */}
        <section>
          <div className="flex items-center justify-between mb-1.5">
            <SectionHeader
              icon={<Search size={14} strokeWidth={1.75} />}
              title="Boolean query"
              description="The actual string you'll paste into each database's advanced search. You'll run the search yourself in your normal browser."
            />
            <button
              type="button"
              onClick={handleSuggest}
              className="no-drag flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-accent-600)] border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)] transition-paper"
            >
              <Sparkles size={12} strokeWidth={2} />
              {query.trim() ? "Advise" : "Suggest"}
            </button>
          </div>
          <div className="mt-4">
            <TextArea
              label="Query"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={handleQueryKeyDown}
              rows={6}
              placeholder={`("social commerce" OR "s-commerce" OR "social shopping")\nAND ("consumer behavi*" OR "buyer behavi*")\nAND ("engagement" OR "purchase" OR "intention")`}
              hint='Use OR between synonyms, AND between concepts. Wildcards (*) where supported. Press Ctrl+Z after accepting a suggestion to undo.'
            />
          </div>
        </section>
```

- [ ] **Step 5: Add the `Sparkles` icon import to the import line**

Find the existing lucide import (line 3):
```tsx
import { Calendar, Database, Globe, Lightbulb, Save, Search } from "lucide-react";
```

Replace with:
```tsx
import { Calendar, Database, Globe, Lightbulb, Save, Search, Sparkles } from "lucide-react";
```

- [ ] **Step 6: Add `QuerySuggestionModal` to the JSX return**

Find the closing `</div>` of the outer return (the last line before `}`). Add the modal just before it:

```tsx
      <QuerySuggestionModal
        open={modalOpen}
        mode={suggestResult === null && !suggestError
          ? (query.trim() ? "advise" : "suggest")
          : (suggestResult !== null || suggestError !== null)
            ? (previousQuery.current !== null || query.trim() ? "advise" : "suggest")
            : "suggest"}
        result={suggestResult}
        error={suggestError}
        onAccept={handleAccept}
        onDismiss={() => { setModalOpen(false); setSuggestResult(null); setSuggestError(null); }}
      />
```

Wait — the `mode` logic above is overly complex. Simplify by storing mode in state. Go back and add a `suggestMode` state:

After `const [suggestError, setSuggestError] = useState<string | null>(null);` add:
```tsx
  const [suggestMode, setSuggestMode] = useState<"suggest" | "advise">("suggest");
```

Update `handleSuggest` to set the mode before calling the API:
```tsx
  const handleSuggest = async () => {
    const mode = query.trim() ? "advise" : "suggest";
    setSuggestMode(mode);
    setSuggestResult(null);
    setSuggestError(null);
    setModalOpen(true);
    try {
      const result = await api.suggestBooleanQuery(
        Array.from(databases),
        query.trim() || null,
      );
      setSuggestResult(result);
    } catch (e: any) {
      setSuggestError(
        e?.message?.includes("chat model")
          ? "Pick a chat model in Settings first."
          : e?.message?.includes("LM Studio") || e?.message?.includes("loopback")
          ? "Could not reach LM Studio — check Settings."
          : "The model returned an unexpected response. Try again.",
      );
    }
  };
```

Then the modal JSX becomes clean:
```tsx
      <QuerySuggestionModal
        open={modalOpen}
        mode={suggestMode}
        result={suggestResult}
        error={suggestError}
        onAccept={handleAccept}
        onDismiss={() => { setModalOpen(false); setSuggestResult(null); setSuggestError(null); }}
      />
```

- [ ] **Step 7: Verify TypeScript compiles**

```bash
npx tsc --noEmit 2>&1 | head -30
```

Expected: no errors.

- [ ] **Step 8: Build and smoke test**

```bash
npm run tauri dev
```

Verify manually:
1. Open a project in Stage 1 · Assembling → Acquisition tab
2. With the query box **empty**: button shows "Suggest" — click it — modal opens with spinner — after model responds, query appears in monospace block with rationale below
3. Click **Accept** — query populates the textarea — modal closes
4. Press **Ctrl+Z** — textarea reverts to empty
5. Type something in the query box: button changes to **"Advise"** — click it — modal shows "AI Advisement" eyebrow
6. Accept the advisement — textarea updates
7. Click **Dismiss** — modal closes, textarea unchanged
8. Press **Escape** with modal open — modal closes

- [ ] **Step 9: Commit**

```bash
git add src/views/assembling/AcquisitionForm.tsx
git commit -m "feat(ui): wire AI Suggest/Advise button and modal into AcquisitionForm"
```

---

## Self-Review

**Spec coverage check:**
- ✅ Button label: "Suggest" (empty) / "Advise" (has content) — Task 5 Step 4
- ✅ Modal with preview, rationale, Accept/Dismiss — Task 4
- ✅ Generate from scratch when empty — Task 1 `suggest_query` with `existing_query: None`
- ✅ Refine existing query — Task 1 `suggest_query` with `existing_query: Some(q)`
- ✅ Ctrl+Z undo via `previousQuery` ref — Task 5 Steps 3 & 4
- ✅ Error handling: LM Studio unreachable, unparseable response — Task 5 Step 3, Task 4
- ✅ Escape to dismiss modal — Task 4 Step 1

**Type consistency check:**
- `SuggestQueryResult` defined in Task 1 (Rust) → Task 3 (TypeScript interface) → Task 4 (Props type) — all consistent
- `suggestBooleanQuery(databases, existingQuery)` in Task 3 matches `SuggestQueryArgs { databases, existing_query }` in Task 2
- `handleAccept(query: string)` in Task 5 matches `onAccept: (query: string) => void` in Task 4

**Placeholder scan:** No TBDs or incomplete steps found.
