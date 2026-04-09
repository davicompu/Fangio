# Boolean Query AI Suggestion — Design Spec

**Date:** 2026-04-09  
**Stage affected:** Stage 1 · Assembling → Acquisition tab  
**Feature:** AI-assisted boolean query generation and advisement

---

## Overview

Add an AI button to the boolean query textarea in `AcquisitionForm`. When clicked, the app calls a local LM Studio model (same infrastructure used by `suggest_codes`) to either generate a query from scratch or improve an existing one. The result appears in a modal where the user can accept or dismiss it.

---

## Architecture

### Backend — `src-tauri/src/suggest.rs`

New function `suggest_query` added alongside the existing `suggest_codes` function.

**Inputs:**
- `base_url: &str`, `model: &str` — LM Studio connection (read from settings)
- `domain: &str` — project domain
- `framework: &str` — organising framework
- `rq_what`, `rq_how`, `rq_where: Option<&str>` — research questions from project row
- `selected_databases: &[String]` — databases checked in the form
- `existing_query: Option<&str>` — `None` = generate from scratch; `Some(q)` = advise/improve

**Behaviour:**
- When `existing_query` is `None`: prompt instructs the model to generate a complete boolean query
- When `existing_query` is `Some(q)`: prompt instructs the model to improve and advise on the existing query

**Output struct `SuggestQueryResult`:**
```rust
pub struct SuggestQueryResult {
    pub query: String,
    pub rationale: String,
    pub model: Option<String>,
    pub latency_ms: u64,
}
```

The model is prompted to return a single JSON object `{ "query": "...", "rationale": "..." }`. The parser extracts the first such object from the response, tolerating prose around it.

### Tauri command — `src-tauri/src/lib.rs`

New command `suggest_boolean_query` registered in `lib.rs`. It:
1. Reads LM Studio `base_url` and `model` from app settings
2. Reads the current project from app state (domain, framework, RQs)
3. Accepts `databases: Vec<String>` and `existing_query: Option<String>` as arguments
4. Calls `suggest_query` and returns `SuggestQueryResult`

### Frontend API — `src/lib/api.ts`

New typed method:
```ts
suggestBooleanQuery(databases: string[], existingQuery: string | null): Promise<SuggestQueryResult>
```

New interface:
```ts
export interface SuggestQueryResult {
  query: string;
  rationale: string;
  model: string | null;
  latency_ms: number;
}
```

---

## Frontend

### Button — `AcquisitionForm.tsx`

- Placed inline with the "Boolean query" section header, to the right of the label
- Icon: `Sparkles` (already imported in the project)
- Label: **"Suggest"** when the textarea is empty; **"Advise"** when it has content
- Disabled when LM Studio is not reachable or no model is selected
- On click: sets `modalOpen = true`, calls `api.suggestBooleanQuery()`

### Modal — `src/components/QuerySuggestionModal.tsx`

New component. Centered overlay using existing card/paper design tokens.

**Contents:**
- Eyebrow: `"AI Suggestion"` (generate mode) or `"AI Advisement"` (advise mode)
- Suggested query in a monospaced read-only code block
- Rationale sentence in muted `text-[var(--color-paper-600)]` text
- Loading state: spinner + `"Thinking…"` while awaiting model response
- Two action buttons: **Accept** (primary) and **Dismiss** (ghost)

**On Accept:**
- Saves the current textarea value to a `previousQuery` ref before overwriting
- Sets the textarea value to the suggested query via React state

**Undo (Ctrl+Z):**
- The textarea has a `keydown` handler that intercepts `Ctrl+Z` / `Cmd+Z`
- If `previousQuery` ref is set, restores it and clears the ref
- Only active after an Accept — does not interfere with normal typing undo

---

## Data Flow

```
User clicks "Suggest"/"Advise"
  → modal opens in loading state
  → api.suggestBooleanQuery(selectedDatabases, query || null)
    → Tauri: suggest_boolean_query command
      → reads project context from app state
      → calls LM Studio via suggest_query()
      → returns SuggestQueryResult
  → modal shows query + rationale
User clicks "Accept"
  → previousQuery ref ← current textarea value
  → textarea value ← suggestion
  → modal closes
User presses Ctrl+Z (optional)
  → textarea value ← previousQuery ref
  → previousQuery ref cleared
```

---

## Error Handling

- If LM Studio is unreachable: show an inline error in the modal (`"Could not reach LM Studio — check Settings."`)
- If model returns unparseable JSON: show `"The model returned an unexpected response. Try again."`
- Modal stays open on error so user doesn't lose context

---

## Out of Scope

- Streaming the response token-by-token (not supported by current LM Studio client)
- History of past suggestions
- Diff view between existing and suggested query
