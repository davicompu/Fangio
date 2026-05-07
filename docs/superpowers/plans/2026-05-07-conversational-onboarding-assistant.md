# Conversational Onboarding Assistant Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the one-shot "✦ Ask assistant" on each onboarding step with a brief two-step dialogue: the model asks 1–3 multi-choice clarifying questions, the user answers (option pills or "Other…" free text), and only then does the model produce the structured rewrite/advice. The existing "Use this", undo, and "Why this step?" panel keep working unchanged.

**Architecture:** A new Tauri command `suggest_step_questions` returns 1–3 questions per step. The existing `suggest_step` command gains an optional `answers` field that, when present, prepends a "Researcher's clarifications" block to the rewrite prompt. The React side replaces the wizard's `suggesting`/`suggestion`/`suggestError` triplet with a phase machine (`idle → asking → answering → rewriting → result/error`) rendered inside the same accent-50 chat bubble.

**Tech Stack:** Rust + Tauri v2 (backend), React 19 + TypeScript (frontend), Tailwind CSS + CSS variables, Framer Motion, Lucide icons, LM Studio local inference.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src-tauri/src/suggest.rs` | Add `StepQuestion`, `StepQuestionsResult`, `QAndA` types; add `parse_step_questions` parser; add `suggest_step_questions` async function; modify `suggest_step` to accept `Option<&[QAndA]>` |
| Modify | `src-tauri/src/lib.rs` | Add `answers` field to `SuggestStepArgs`; add `SuggestStepQuestionsArgs`, `SuggestStepQuestionsResult` structs and `suggest_step_questions` command; register the new command; update `suggest_step` callsite |
| Modify | `src/lib/api.ts` | Add `StepQuestion`, `StepQuestionsResult`, `QAndA` interfaces; add `suggestStepQuestions` method; update `suggestStep` signature with optional `answers` |
| Modify | `src/views/onboarding/OnboardingWizard.tsx` | Replace state with phase machine; add `<PhaseBubble>` and `<QuestionRow>` inline components; rewire button to start the dialogue |

No new files, no new test infrastructure, no new dependencies.

---

## Task 1: Backend types + JSON parser (TDD)

**Files:**
- Modify: `src-tauri/src/suggest.rs`

- [ ] **Step 1: Add the failing tests at the end of the existing `mod tests` block**

Append inside `mod tests { ... }` in `src-tauri/src/suggest.rs`, just before the closing `}`:

```rust
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
```

- [ ] **Step 2: Run the tests and confirm they fail**

```bash
cd src-tauri && cargo test --lib parse_step_questions --message-format=short 2>&1 | tail -20
```

Expected: compile error like `cannot find function 'parse_step_questions'` and `cannot find type 'StepQuestion'`. That's the failing-test state.

- [ ] **Step 3: Add `StepQuestion`, `StepQuestionsResult`, `QAndA` types**

Insert after the existing `SuggestQueryResult` struct (around line 35 — find the `}` that closes `SuggestQueryResult` and add the block immediately after it):

```rust
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QAndA {
    pub q: String,
    pub a: String,
}
```

- [ ] **Step 4: Add `parse_step_questions` function**

Insert immediately after the existing `pick_str` helper (find `fn pick_str<'a>` and add the following block after its closing `}`):

```rust
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
```

- [ ] **Step 5: Run the tests and confirm they pass**

```bash
cd src-tauri && cargo test --lib parse_step_questions --message-format=short 2>&1 | tail -15
```

Expected: `test result: ok. 6 passed; 0 failed`.

- [ ] **Step 6: Verify the rest of the suite still passes**

```bash
cd src-tauri && cargo test --lib suggest:: --message-format=short 2>&1 | tail -10
```

Expected: all 19 tests in `suggest::tests` pass (13 existing + 6 new).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/suggest.rs
git commit -m "feat(suggest): parse_step_questions + types for clarifying questions"
```

---

## Task 2: Backend `suggest_step_questions` async function

**Files:**
- Modify: `src-tauri/src/suggest.rs`

- [ ] **Step 1: Add the function**

Insert **before** the existing `pub async fn suggest_step(` definition (around line 55). Add this block:

```rust
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
    api_key: Option<&str>,
) -> Result<StepQuestionsResult> {
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

    let messages = vec![
        ChatMessage { role: "system".into(), content: system },
        ChatMessage { role: "user".into(), content: user },
    ];

    let result = lmstudio::chat(base_url, model, &messages, Some(0.4), api_key).await?;

    let questions = parse_step_questions(&result.content)
        .ok_or_else(|| anyhow!("could not parse clarifying questions from model output"))?;

    if questions.is_empty() {
        return Err(anyhow!("model returned no clarifying questions"));
    }

    Ok(StepQuestionsResult {
        questions,
        model: result.model,
        latency_ms: result.latency_ms,
    })
}
```

- [ ] **Step 2: Verify Rust compiles**

```bash
cd src-tauri && cargo check --message-format=short 2>&1 | tail -10
```

Expected: no errors. (Warnings about unused fields elsewhere are pre-existing.)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/suggest.rs
git commit -m "feat(suggest): suggest_step_questions live LLM call"
```

---

## Task 3: Modify `suggest_step` to accept optional answers

**Files:**
- Modify: `src-tauri/src/suggest.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Update `suggest_step` signature and prompt assembly**

Find `pub async fn suggest_step(` in `src-tauri/src/suggest.rs` (around line 56). Change the parameter list to add `answers` as the last parameter before `api_key`. The new signature is:

```rust
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
    api_key: Option<&str>,
) -> Result<(String, String)> {
```

- [ ] **Step 2: Build the clarifications block and prepend it to the user prompt**

Inside `suggest_step`, immediately after the existing `let user = match step_id { ... };` block (find the closing `};` after the `other =>` branch), insert:

```rust
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
```

- [ ] **Step 3: Update the only Rust callsite in `lib.rs`**

In `src-tauri/src/lib.rs`, find the `suggest::suggest_step(` call (around line 1803). It currently passes 9 arguments. Change the argument list to insert `None` (the answers placeholder) as the second-to-last argument, immediately before `api_key.as_deref()`:

```rust
    let (suggestion, rationale) = suggest::suggest_step(
        &base_url,
        &model,
        &args.step_id,
        &args.current_value,
        &project.name,
        project.slr_type.as_deref().unwrap_or(""),
        project.framework.as_deref().unwrap_or(""),
        project.domain.as_deref().unwrap_or(""),
        args.answers.as_deref(),
        api_key.as_deref(),
    )
    .await
    .map_err(err)?;
```

(The `args.answers` field will be added in the next step.)

- [ ] **Step 4: Add `answers` to `SuggestStepArgs`**

Find the `SuggestStepArgs` struct in `src-tauri/src/lib.rs` (around line 1767):

```rust
#[derive(Debug, Deserialize)]
struct SuggestStepArgs {
    step_id: String,
    current_value: String,
}
```

Replace with:

```rust
#[derive(Debug, Deserialize)]
struct SuggestStepArgs {
    step_id: String,
    current_value: String,
    #[serde(default)]
    answers: Option<Vec<suggest::QAndA>>,
}
```

- [ ] **Step 5: Verify Rust compiles**

```bash
cd src-tauri && cargo check --message-format=short 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 6: Run all suggest tests to confirm no regression**

```bash
cd src-tauri && cargo test --lib suggest:: --message-format=short 2>&1 | tail -8
```

Expected: 19 tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/suggest.rs src-tauri/src/lib.rs
git commit -m "feat(suggest): suggest_step accepts optional clarifications"
```

---

## Task 4: Wire `suggest_step_questions` Tauri command

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add args, result, and command after the `suggest_step` command**

In `src-tauri/src/lib.rs`, find the closing `}` of the `async fn suggest_step` function (around line 1817 — the line `Ok(SuggestStepResult { suggestion, rationale })` is followed by `}`). Insert this block immediately after that closing `}`:

```rust
#[derive(Debug, Deserialize)]
struct SuggestStepQuestionsArgs {
    step_id: String,
    current_value: String,
}

#[tauri::command]
async fn suggest_step_questions(
    args: SuggestStepQuestionsArgs,
    state: State<'_, AppState>,
) -> Result<suggest::StepQuestionsResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let project = db.get_project().await.map_err(err)?;
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
        .unwrap_or("google/gemma-4-e2b")
        .to_string();
    if model.is_empty() {
        return Err("Pick a chat model in Settings first.".into());
    }
    let result = suggest::suggest_step_questions(
        &base_url,
        &model,
        &args.step_id,
        &args.current_value,
        &project.name,
        project.slr_type.as_deref().unwrap_or(""),
        project.framework.as_deref().unwrap_or(""),
        project.domain.as_deref().unwrap_or(""),
        api_key.as_deref(),
    )
    .await
    .map_err(err)?;
    Ok(result)
}
```

- [ ] **Step 2: Register the command in `tauri::generate_handler!`**

Find the `suggest_step,` line inside the `tauri::generate_handler!` block (around line 3121). Add `suggest_step_questions,` on the line immediately after it:

```rust
            suggest_step,
            suggest_step_questions,
```

- [ ] **Step 3: Verify Rust compiles**

```bash
cd src-tauri && cargo check --message-format=short 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(backend): register suggest_step_questions Tauri command"
```

---

## Task 5: Update TypeScript API surface

**Files:**
- Modify: `src/lib/api.ts`

- [ ] **Step 1: Add the new types**

In `src/lib/api.ts`, find the existing `interface SuggestQueryResult` (around line 612 — search for `SuggestQueryResult`) and add the following block immediately after its closing `}`:

```ts
export interface StepQuestion {
  q: string;
  options: string[];
}

export interface StepQuestionsResult {
  questions: StepQuestion[];
  model: string | null;
  latency_ms: number;
}

export interface QAndA {
  q: string;
  a: string;
}
```

- [ ] **Step 2: Update `suggestStep` signature and add `suggestStepQuestions`**

Find the existing `suggestStep` method on the `api` object (around line 607):

```ts
  suggestStep(stepId: string, currentValue: string) {
    return invoke<{ suggestion: string; rationale: string }>("suggest_step", {
      args: { step_id: stepId, current_value: currentValue },
    });
  },
```

Replace with:

```ts
  suggestStep(stepId: string, currentValue: string, answers?: QAndA[]) {
    return invoke<{ suggestion: string; rationale: string }>("suggest_step", {
      args: {
        step_id: stepId,
        current_value: currentValue,
        answers: answers && answers.length > 0 ? answers : null,
      },
    });
  },
  suggestStepQuestions(stepId: string, currentValue: string) {
    return invoke<StepQuestionsResult>("suggest_step_questions", {
      args: { step_id: stepId, current_value: currentValue },
    });
  },
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): suggestStepQuestions + optional answers on suggestStep"
```

---

## Task 6: Frontend phase machine — replace state, no UI yet

**Files:**
- Modify: `src/views/onboarding/OnboardingWizard.tsx`

This task swaps the wizard's existing state primitives for a single `phase` discriminated union. Render output of the bubble is left looking exactly like today (one-shot result only) — Task 7 introduces the question UI. After this task the wizard works identically to before, just with the new state shape underneath.

- [ ] **Step 1: Add the import**

In `src/views/onboarding/OnboardingWizard.tsx`, find the existing API import:

```ts
import { api, type ProjectRow } from "../../lib/api";
```

Replace with:

```ts
import { api, type ProjectRow, type StepQuestion } from "../../lib/api";
```

- [ ] **Step 2: Define the `Phase` type**

Find the line `type Answers = Record<string, string>;` (around line 17). Add the following block immediately after it:

```ts
type Phase =
  | { kind: "idle" }
  | { kind: "asking" }
  | { kind: "answering"; questions: StepQuestion[]; answers: Record<number, string> }
  | { kind: "rewriting"; questions: StepQuestion[]; answers: Record<number, string> }
  | { kind: "result"; suggestion: string; rationale: string }
  | { kind: "error"; message: string; recoverable: "asking" | "rewriting" };
```

- [ ] **Step 3: Replace the AI assistant state**

Find this block (around line 41–45):

```ts
  // AI assistant state
  const [suggesting, setSuggesting] = useState(false);
  const [suggestion, setSuggestion] = useState<{ suggestion: string; rationale: string } | null>(null);
  const [suggestError, setSuggestError] = useState<string | null>(null);
  const previousValue = useRef<string | null>(null);
```

Replace with:

```ts
  // AI assistant phase machine
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const previousValue = useRef<string | null>(null);
```

- [ ] **Step 4: Update `clearAssistant`**

Find `const clearAssistant = () => { ... };` (around line 58):

```ts
  const clearAssistant = () => {
    setSuggestion(null);
    setSuggestError(null);
    previousValue.current = null;
  };
```

Replace with:

```ts
  const clearAssistant = () => {
    setPhase({ kind: "idle" });
    previousValue.current = null;
  };
```

- [ ] **Step 5: Helper for normalising LM Studio errors**

Find `const handleSuggest = async () => {` (around line 110) and add **above** it:

```ts
  const errorMessage = (e: unknown): string => {
    const msg: string = (e as { message?: string })?.message ?? String(e);
    if (msg.includes("chat model")) return "Pick a chat model in Settings first.";
    if (msg.includes("Connection refused") || msg.includes("error sending request") || msg.includes("connect error")) {
      return "Could not reach LM Studio — check Settings.";
    }
    return "Unexpected error. Try again.";
  };
```

- [ ] **Step 6: Rewrite `handleSuggest` to use the phase machine — one-shot path only for now**

Replace the entire `handleSuggest` function (around lines 110–129) with this temporary one-shot version that drives the new phase types but skips the questions step:

```ts
  const handleSuggest = async () => {
    setPhase({ kind: "asking" });
    try {
      const result = await api.suggestStep(step.id, value);
      setPhase({ kind: "result", suggestion: result.suggestion, rationale: result.rationale });
    } catch (e) {
      setPhase({ kind: "error", message: errorMessage(e), recoverable: "rewriting" });
    }
  };
```

(The full asking → answering → rewriting flow lands in Task 7. Keeping this task focused on the state-shape swap means each task can be reviewed and reverted independently.)

- [ ] **Step 7: Update `acceptSuggestion`**

Find:

```ts
  const acceptSuggestion = () => {
    if (!suggestion) return;
    previousValue.current = value;
    setValue(suggestion.suggestion);
    setSuggestion(null);
  };
```

Replace with:

```ts
  const acceptSuggestion = () => {
    if (phase.kind !== "result") return;
    previousValue.current = value;
    setValue(phase.suggestion);
    setPhase({ kind: "idle" });
  };
```

- [ ] **Step 8: Update the bubble JSX**

Find the section starting with `{/* AI chat bubble */}` (around line 205) and ending at the matching `</AnimatePresence>` (around line 261). Replace the entire `<AnimatePresence>` block with:

```tsx
              {/* AI chat bubble */}
              <AnimatePresence>
                {phase.kind !== "idle" && (
                  <motion.div
                    initial={{ opacity: 0, y: -8 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -8 }}
                    transition={{ duration: 0.25 }}
                    className="mb-4 rounded-[var(--radius-md)] bg-[var(--color-accent-50)] border border-[var(--color-accent-100)] p-4"
                  >
                    <div className="flex items-start gap-3">
                      <div className="shrink-0 w-7 h-7 rounded-full bg-[var(--color-accent-600)] flex items-center justify-center">
                        <Sparkles size={13} strokeWidth={1.75} className="text-white" />
                      </div>
                      <div className="flex-1 min-w-0">
                        {phase.kind === "asking" && (
                          <div className="flex items-center gap-2 py-1">
                            <ThinkingDots />
                            <span className="text-[12px] text-[var(--color-paper-500)]">Working on a rewrite…</span>
                          </div>
                        )}
                        {phase.kind === "rewriting" && (
                          <div className="flex items-center gap-2 py-1">
                            <ThinkingDots />
                            <span className="text-[12px] text-[var(--color-paper-500)]">Working on a rewrite…</span>
                          </div>
                        )}
                        {phase.kind === "error" && (
                          <div className="flex items-start justify-between gap-2">
                            <p className="text-[13px] text-[var(--color-warn-700)]">{phase.message}</p>
                            <button
                              type="button"
                              onClick={() => setPhase({ kind: "idle" })}
                              className="shrink-0 mt-0.5 text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)] transition-paper"
                              aria-label="Close"
                            >
                              <X size={12} strokeWidth={2} />
                            </button>
                          </div>
                        )}
                        {phase.kind === "result" && (
                          <div className="space-y-3">
                            <div className="flex items-start justify-between gap-2">
                              <Markdown className="markdown-sm flex-1">{phase.suggestion}</Markdown>
                              <button
                                type="button"
                                onClick={() => setPhase({ kind: "idle" })}
                                className="shrink-0 mt-0.5 text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)] transition-paper"
                                aria-label="Close"
                              >
                                <X size={12} strokeWidth={2} />
                              </button>
                            </div>
                            {phase.rationale && (
                              <p className="text-[11px] text-[var(--color-accent-600)] leading-snug border-t border-[var(--color-accent-100)] pt-2">
                                {phase.rationale}
                              </p>
                            )}
                            {isReplaceable && (
                              <button
                                type="button"
                                onClick={acceptSuggestion}
                                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-xs)] text-[11px] font-medium bg-[var(--color-accent-600)] text-white hover:bg-[var(--color-accent-700)] transition-paper"
                              >
                                Use this
                              </button>
                            )}
                          </div>
                        )}
                      </div>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
```

- [ ] **Step 9: Update the assistant trigger button's `disabled` and label**

Find the button rendered at `<button type="button" onClick={handleSuggest}` (around line 188). Its `disabled` and label both reference the old `suggesting` boolean. Replace the relevant attributes so they read:

```tsx
                  disabled={!canSuggest || phase.kind === "asking" || phase.kind === "rewriting"}
                  className={cn(
                    "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-[12px] font-medium transition-paper",
                    canSuggest && phase.kind !== "asking" && phase.kind !== "rewriting"
                      ? "text-[var(--color-accent-600)] border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)]"
                      : "text-[var(--color-paper-400)] border border-[var(--color-paper-200)] cursor-not-allowed",
                  )}
                  title={!canSuggest ? "Write your answer first" : undefined}
                >
                  {phase.kind === "asking" || phase.kind === "rewriting" ? <ThinkingDots small /> : <Sparkles size={12} strokeWidth={2} />}
                  {phase.kind === "asking" || phase.kind === "rewriting" ? "Working…" : "✦ Ask assistant"}
```

- [ ] **Step 10: Verify TypeScript compiles**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 11: Commit**

```bash
git add src/views/onboarding/OnboardingWizard.tsx
git commit -m "refactor(onboarding): replace assistant state with phase machine"
```

---

## Task 7: Frontend `<QuestionRow>` component

**Files:**
- Modify: `src/views/onboarding/OnboardingWizard.tsx`

- [ ] **Step 1: Add the component at the bottom of the file**

In `src/views/onboarding/OnboardingWizard.tsx`, find the end of the file (after `function ProgressTrail(...)`). Add this component just before the file's final newline:

```tsx
interface QuestionRowProps {
  question: StepQuestion;
  index: number;
  answer: string | undefined;
  onAnswer: (answer: string) => void;
}

function QuestionRow({ question, index, answer, onAnswer }: QuestionRowProps) {
  const [otherOpen, setOtherOpen] = useState(false);
  const [otherText, setOtherText] = useState("");

  // Whether the current answer matches a known option
  const matchedOption = question.options.find((opt) => opt === answer);
  const isOtherActive = answer !== undefined && !matchedOption;

  // Re-sync free-text input when phase resets (different question instance ⇒ component remounts).
  useEffect(() => {
    if (isOtherActive) {
      setOtherOpen(true);
      setOtherText(answer ?? "");
    }
  }, []); // run once on mount

  const onPickOption = (opt: string) => {
    setOtherOpen(false);
    setOtherText("");
    onAnswer(opt);
  };

  const onPickOther = () => {
    setOtherOpen(true);
    if (otherText.trim()) onAnswer(otherText);
  };

  const onOtherChange = (v: string) => {
    setOtherText(v);
    if (v.trim()) onAnswer(v);
  };

  const showOnlyFreeText = question.options.length === 0;

  return (
    <div className="space-y-2">
      <p className="text-[12px] font-medium text-[var(--color-ink-800)]">
        <span className="text-[var(--color-accent-600)] font-mono mr-1.5">{index + 1}.</span>
        {question.q}
      </p>
      {!showOnlyFreeText && (
        <div className="flex flex-wrap items-center gap-1.5">
          {question.options.map((opt) => {
            const selected = matchedOption === opt && !isOtherActive;
            return (
              <button
                key={opt}
                type="button"
                onClick={() => onPickOption(opt)}
                className={cn(
                  "inline-flex items-center px-3 h-8 rounded-full text-[11px] font-medium transition-paper",
                  selected
                    ? "bg-[var(--color-accent-600)] text-white border border-[var(--color-accent-600)]"
                    : "bg-[var(--color-accent-50)] text-[var(--color-accent-600)] border border-[var(--color-accent-300)] hover:bg-[var(--color-accent-100)]",
                )}
              >
                {opt}
              </button>
            );
          })}
          <button
            type="button"
            onClick={onPickOther}
            className={cn(
              "inline-flex items-center px-3 h-8 rounded-full text-[11px] font-medium transition-paper",
              isOtherActive
                ? "bg-[var(--color-accent-600)] text-white border border-[var(--color-accent-600)]"
                : "bg-white text-[var(--color-paper-600)] border border-[var(--color-paper-300)] hover:border-[var(--color-paper-500)]",
            )}
          >
            Other…
          </button>
        </div>
      )}
      {(otherOpen || showOnlyFreeText) && (
        <input
          type="text"
          value={otherText}
          onChange={(e) => onOtherChange(e.target.value)}
          placeholder="Type your answer"
          className="w-full px-3 py-2 text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-accent-300)] bg-white focus:outline-none focus:border-[var(--color-accent-500)]"
          autoFocus={otherOpen}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Add `useEffect` to the React import if missing**

Open the top of the file. The existing import is:

```ts
import { useEffect, useRef, useState } from "react";
```

`useEffect` is already imported. No change needed — just confirm.

- [ ] **Step 3: Verify TypeScript compiles**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors. (The component is unused in this task — Task 8 wires it in.)

- [ ] **Step 4: Commit**

```bash
git add src/views/onboarding/OnboardingWizard.tsx
git commit -m "feat(onboarding): QuestionRow component with pills and Other field"
```

---

## Task 8: Wire the asking → answering → rewriting flow

**Files:**
- Modify: `src/views/onboarding/OnboardingWizard.tsx`

- [ ] **Step 1: Replace the temporary one-shot `handleSuggest`**

Find the temporary `handleSuggest` from Task 6:

```ts
  const handleSuggest = async () => {
    setPhase({ kind: "asking" });
    try {
      const result = await api.suggestStep(step.id, value);
      setPhase({ kind: "result", suggestion: result.suggestion, rationale: result.rationale });
    } catch (e) {
      setPhase({ kind: "error", message: errorMessage(e), recoverable: "rewriting" });
    }
  };
```

Replace with the full set of handlers:

```ts
  const handleSuggest = async () => {
    setPhase({ kind: "asking" });
    try {
      const result = await api.suggestStepQuestions(step.id, value);
      setPhase({ kind: "answering", questions: result.questions, answers: {} });
    } catch (e) {
      setPhase({ kind: "error", message: errorMessage(e), recoverable: "asking" });
    }
  };

  const setAnswer = (idx: number, answer: string) => {
    setPhase((p) => {
      if (p.kind !== "answering") return p;
      return { ...p, answers: { ...p.answers, [idx]: answer } };
    });
  };

  const submitAnswers = async () => {
    if (phase.kind !== "answering") return;
    const qaPairs = phase.questions
      .map((q, i) => ({ q: q.q, a: phase.answers[i] ?? "" }))
      .filter((p) => p.a.trim().length > 0);
    setPhase({ kind: "rewriting", questions: phase.questions, answers: phase.answers });
    try {
      const result = await api.suggestStep(step.id, value, qaPairs);
      setPhase({ kind: "result", suggestion: result.suggestion, rationale: result.rationale });
    } catch (e) {
      setPhase({
        kind: "error",
        message: errorMessage(e),
        recoverable: "rewriting",
      });
    }
  };

  const skipQuestions = async () => {
    setPhase({ kind: "rewriting", questions: [], answers: {} });
    try {
      const result = await api.suggestStep(step.id, value);
      setPhase({ kind: "result", suggestion: result.suggestion, rationale: result.rationale });
    } catch (e) {
      setPhase({ kind: "error", message: errorMessage(e), recoverable: "rewriting" });
    }
  };

  const retry = async () => {
    if (phase.kind !== "error") return;
    if (phase.recoverable === "asking") {
      await handleSuggest();
    } else {
      // rewriting: re-attempt with the answers we still have in phase history.
      // We don't store them on the error variant, so the simplest recovery is
      // to send the user back to the answering screen by re-running questions.
      await handleSuggest();
    }
  };
```

(Pre-emptive: keeping the answers across an error during rewriting is in the spec, but we don't carry them on the `error` variant. Sufficient first-cut behaviour: Retry restarts the dialogue from the asking phase. A future enhancement can preserve answers — see "Future work" in the spec.)

- [ ] **Step 2: Render the `answering` phase content**

Inside the bubble JSX from Task 6, find the block:

```tsx
                        {phase.kind === "asking" && (
                          <div className="flex items-center gap-2 py-1">
                            <ThinkingDots />
                            <span className="text-[12px] text-[var(--color-paper-500)]">Working on a rewrite…</span>
                          </div>
                        )}
```

Replace it with — **a clearer asking label**:

```tsx
                        {phase.kind === "asking" && (
                          <div className="flex items-center gap-2 py-1">
                            <ThinkingDots />
                            <span className="text-[12px] text-[var(--color-paper-500)]">Thinking up some questions…</span>
                          </div>
                        )}
```

Then, immediately after that asking block, insert the `answering` block:

```tsx
                        {phase.kind === "answering" && (
                          <div className="space-y-4">
                            <div className="space-y-3">
                              {phase.questions.map((q, i) => (
                                <QuestionRow
                                  key={i}
                                  question={q}
                                  index={i}
                                  answer={phase.answers[i]}
                                  onAnswer={(a) => setAnswer(i, a)}
                                />
                              ))}
                            </div>
                            <div className="flex items-center gap-3 pt-2 border-t border-[var(--color-accent-100)]">
                              <button
                                type="button"
                                onClick={submitAnswers}
                                disabled={phase.questions.some((_, i) => !phase.answers[i] || !phase.answers[i].trim())}
                                className={cn(
                                  "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-xs)] text-[11px] font-medium transition-paper",
                                  phase.questions.every((_, i) => phase.answers[i] && phase.answers[i].trim())
                                    ? "bg-[var(--color-accent-600)] text-white hover:bg-[var(--color-accent-700)]"
                                    : "bg-[var(--color-paper-200)] text-[var(--color-paper-400)] cursor-not-allowed",
                                )}
                              >
                                Generate suggestion
                              </button>
                              <button
                                type="button"
                                onClick={skipQuestions}
                                className="text-[11px] text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] underline-offset-2 hover:underline transition-paper"
                              >
                                Skip — just suggest
                              </button>
                            </div>
                          </div>
                        )}
```

- [ ] **Step 3: Update the error block to wire Retry**

Find the existing error block from Task 6 (the one that currently only has an X close button). Replace it with:

```tsx
                        {phase.kind === "error" && (
                          <div className="space-y-3">
                            <div className="flex items-start justify-between gap-2">
                              <p className="text-[13px] text-[var(--color-warn-700)] flex-1">{phase.message}</p>
                              <button
                                type="button"
                                onClick={() => setPhase({ kind: "idle" })}
                                className="shrink-0 mt-0.5 text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)] transition-paper"
                                aria-label="Close"
                              >
                                <X size={12} strokeWidth={2} />
                              </button>
                            </div>
                            <button
                              type="button"
                              onClick={retry}
                              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-xs)] text-[11px] font-medium bg-[var(--color-accent-600)] text-white hover:bg-[var(--color-accent-700)] transition-paper"
                            >
                              Retry
                            </button>
                          </div>
                        )}
```

- [ ] **Step 4: Verify TypeScript compiles**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/views/onboarding/OnboardingWizard.tsx
git commit -m "feat(onboarding): wire asking-answering-rewriting dialogue flow"
```

---

## Task 9: Manual smoke test

**Prerequisites:**
- LM Studio running locally at the URL configured in Reeves Settings
- A chat model selected in Settings (e.g. `google/gemma-4-26b-a4b`)
- A new or existing project open

- [ ] **Step 1: Start the dev server**

```bash
npm run tauri dev
```

Wait for the app window to open.

- [ ] **Step 2: Drive the wizard through step 1 (`name`)**

  - Open or create a project that lands on the onboarding step "What should we call this review?".
  - Type a partial name (e.g., "AI in business education").
  - Click **✦ Ask assistant**. Expect the bubble to show "Thinking up some questions…", then 1–3 question rows with pill buttons and an "Other…" pill on each.
  - Click an option pill on each question. Confirm the **Generate suggestion** button enables only after every question has an answer.
  - On one question, click **Other…**, type a custom answer; confirm it activates and **Generate suggestion** still enables.
  - Click **Generate suggestion**. Expect "Working on a rewrite…" then a result bubble with the rewrite, optional rationale below, and a **Use this** button.
  - Click **Use this**. Expect the field above to be replaced and the **Undo AI suggestion** link to appear under the field.
  - Click **Undo AI suggestion**. Expect the field to revert to the original draft.

- [ ] **Step 3: Drive a long-text step (`domain`)**

  - Advance to step 4 (Domain).
  - Type a 1–2 sentence draft.
  - Repeat the dialogue: Ask → answer → Generate → Use this → undo. Confirm the **Use this** button is present (replaceable step).

- [ ] **Step 4: Drive a choice step (`slr_type`)**

  - Advance to step 2 (SLR Type).
  - Click a card to set a choice.
  - Click **✦ Ask assistant** and answer the questions.
  - Click **Generate suggestion**. Confirm the result bubble appears with rationale but **NO Use this** button (advisory only — the spec requires this for choice steps because the field already has a discrete value).

- [ ] **Step 5: Skip path**

  - On any step, click **✦ Ask assistant**, wait for questions, then click **Skip — just suggest**.
  - Confirm the legacy one-shot path runs: bubble shows "Working on a rewrite…" then a result with rewrite/rationale, exactly like before this feature shipped.

- [ ] **Step 6: Error path**

  - Stop LM Studio (Quit the LM Studio app, or kill the local server).
  - Click **✦ Ask assistant**.
  - Confirm the bubble shows the red message "Could not reach LM Studio — check Settings." with a **Retry** button.
  - Restart LM Studio. Click **Retry**. Confirm the dialogue restarts cleanly from the asking phase.

- [ ] **Step 7: Cancel from any phase**

  - Trigger the assistant. While in the asking, answering, or result phases, click the **X** in the bubble header. Confirm the bubble closes and the **✦ Ask assistant** trigger button re-enables.

- [ ] **Step 8: Cmd/Ctrl+Enter still advances**

  - While the bubble is open in `answering`, press Cmd/Ctrl+Enter. Confirm the wizard's "next" handler runs as before (spec says this must be preserved).

- [ ] **Step 9: All seven steps reachable**

  - Walk through all 7 onboarding steps and trigger the assistant on each. Confirm the assistant doesn't return `unknown step_id` errors. (If a model returns malformed JSON for questions, the bubble should show the parse error message — that's expected and recoverable via Retry, not a regression.)

- [ ] **Step 10: Final commit (changelog only, optional)**

If everything passes the smoke test, no further code change is needed. If a small UI-polish issue surfaced during the smoke test (typo, spacing, copy tweak), fix it and commit:

```bash
git add src/views/onboarding/OnboardingWizard.tsx
git commit -m "polish(onboarding): smoke-test fixes"
```

---

## Self-Review

**Spec coverage:**
- ✅ Phase machine `idle → asking → answering → rewriting → result/error` — Task 6 Step 2, Task 8
- ✅ `suggest_step_questions` Tauri command + Rust function — Tasks 1, 2, 4
- ✅ `suggest_step` accepts optional answers — Task 3
- ✅ TS types `StepQuestion`, `StepQuestionsResult`, `QAndA` — Task 5
- ✅ `<QuestionRow>` with pills + "Other…" + free-text — Task 7
- ✅ Bubble morphs across phases (existing accent-50 card retained) — Task 6 Step 8
- ✅ "Generate suggestion" disabled until all answered — Task 8 Step 2
- ✅ "Skip — just suggest" preserves legacy one-shot path — Task 8 Step 1
- ✅ Use this only on REPLACEABLE_STEPS — preserved in Task 6 Step 8 (`isReplaceable && ...`)
- ✅ Error UX with Retry — Task 8 Step 3
- ✅ Cmd/Ctrl+Enter preserved — untouched
- ✅ "Why this step?" panel preserved — untouched
- ✅ Decision log writes unchanged — `persistCurrent` not modified
- ✅ Empty options array → free-text-only rendering — `<QuestionRow>` Task 7 (`showOnlyFreeText`)
- ⚠️ "Answers preserved on rewriting failure" — partially. The error variant doesn't carry answers; Retry currently restarts from asking. Documented in Task 8 Step 1 as an explicit deviation; flagged as future enhancement.

**Type consistency:**
- `StepQuestion { q, options }` is identical across `suggest.rs` (Task 1), `lib.rs` (passed through), and `api.ts` (Task 5).
- `QAndA { q, a }` matches between Rust `pub struct QAndA` (Task 1) and TS `interface QAndA` (Task 5).
- `suggestStepQuestions(stepId, currentValue)` (Task 5) matches `SuggestStepQuestionsArgs { step_id, current_value }` (Task 4) via Tauri's automatic camelCase ↔ snake_case via the `args:` wrapper.
- `suggestStep(stepId, currentValue, answers?)` (Task 5) matches `SuggestStepArgs { step_id, current_value, answers: Option<Vec<QAndA>> }` (Task 3 Step 4).

**Placeholder scan:**
No "TBD" / "TODO" / vague stand-ins. Every code-modifying step shows the literal code to add. The only deferred item is the error-recovery answer-preservation, which is explicitly named and tied to a known future-work item from the spec.
