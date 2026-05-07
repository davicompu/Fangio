# Conversational Onboarding Assistant — Design

**Status:** approved (2026-05-07)

## Problem

The current onboarding wizard has an "✦ Ask assistant" button on each of its 7 steps. It is one-shot: the user types something, clicks the button, and the model returns a single replacement plus a one-sentence rationale. The user then accepts or dismisses.

This produces low value because the assistant has no chance to clarify intent before generating. It either guesses well or doesn't, and the user gets no insight into how the assistant interpreted their draft. There is no dialogue, no summary of what the user said, no organization of fuzzy thinking — just a take-it-or-leave-it dump.

## Goal

Turn each onboarding-step assistant into a brief two-step dialogue: the assistant asks 1–3 targeted clarifying questions, the user answers, and only then does the assistant produce an organized rewrite with rationale.

The conversation is short (one round), grounded in the user's draft, and resolved inside the same in-place bubble that exists today. The result UI is unchanged — the rewrite still appears as a chat bubble with rationale and a "Use this" button, and Ctrl-Z-style undo still works.

## Non-goals

- Multi-turn refinement after the rewrite. (Future: see "Future work".)
- Persisting the dialogue history to the decision log.
- Streaming responses.
- Replacing the right-side "Why this step?" panel.

## Approach: in-place bubble expansion

The existing AI chat bubble in the main column becomes phase-driven. The same visual element — the rounded accent-50 card with the Sparkles avatar — morphs through five phases:

```
idle → asking → answering → rewriting → result
                                ↘ error (retry returns to asking or rewriting)
```

| Phase | Bubble content |
|-------|----------------|
| `idle` | Hidden (button visible above) |
| `asking` | Spinner + "Thinking up some questions…" |
| `answering` | 1–3 question rows. Each row: question label + horizontal strip of pill buttons (one per option) + "Other…" pill that reveals inline `TextField`. Bottom CTA: **Generate suggestion** (disabled until all questions answered). Secondary link: **Skip — just suggest** (one-shot fallback). |
| `rewriting` | Spinner + "Working on a rewrite…" |
| `result` | Same as today — rewrite markdown, rationale, **Use this** (replaceable steps only), X to dismiss |
| `error` | Red message + Retry button |

A close (X) icon in the bubble header returns the user to `idle` from any phase.

### Behavior across step kinds

- **Long-text steps** (`domain`, `rq_what`, `rq_how`, `rq_where`) and **name**: full flow, result is replaceable (Use this).
- **Choice steps** (`slr_type`, `framework`): same flow but the rewrite is advisory only (no Use this button — same as today). Questions help the user decide between option cards rather than rewrite anything. The model is told the chosen value and the available options, and is asked to question whether the choice fits the user's situation.

## Backend changes

### New: `suggest_step_questions` Tauri command

Returns 1–3 multi-choice questions tailored to the step + the user's current draft.

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StepQuestion {
    pub q: String,
    pub options: Vec<String>,  // 2–4 options
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StepQuestionsResult {
    pub questions: Vec<StepQuestion>,
    pub model: Option<String>,
    pub latency_ms: u64,
}

pub async fn suggest_step_questions(
    base_url: &str,
    model: &str,
    step_id: &str,
    current_value: &str,
    project_context: &ProjectContext,  // existing answered fields
) -> Result<StepQuestionsResult>
```

Prompt (system + user) instructs the model to:
- Read the user's draft for the step.
- Identify the 1–3 most useful clarifying questions for this kind of decision.
- For each question, supply 2–4 plausible options. If a question genuinely needs free text, supply 0 options (the UI will render a single "Other…" field for it).
- Return strict JSON: `{ "questions": [{ "q": "...", "options": ["..."] }, ...] }`.

The Tauri command wires it through `AppState`, mirroring the existing `suggest_step` command shape (load LM Studio config, project context, error handling).

### Modify: `suggest_step` to accept optional answers

Extend the existing function with an optional `answers: Option<&[QAndA]>` parameter:

```rust
pub struct QAndA {
    pub q: String,
    pub a: String,
}
```

When `answers` is `None`, behavior is unchanged (preserves the legacy "Skip — just suggest" path). When present, the prompt prepends a "Researcher's clarifications" block before the rewrite instruction, formatted as `Q: …\nA: …` per pair.

The Tauri command `suggest_step` is updated to accept an optional `answers` field on its args struct.

### Files

| Action | Path | Purpose |
|--------|------|---------|
| Modify | `src-tauri/src/suggest.rs` | Add `StepQuestion`, `StepQuestionsResult`, `QAndA` types; add `suggest_step_questions` function; modify `suggest_step` to take `Option<&[QAndA]>` |
| Modify | `src-tauri/src/lib.rs` | Add `suggest_step_questions` command; extend `SuggestStepArgs` with optional `answers`; register new command |
| Modify | `src/lib/api.ts` | Add `StepQuestion`, `StepQuestionsResult`, `QAndA` types; add `suggestStepQuestions` method; update `suggestStep` signature |

## Frontend changes

### `OnboardingWizard.tsx` state

Replace the existing `suggesting` / `suggestion` / `suggestError` triplet with a phase machine:

```ts
type Phase =
  | { kind: "idle" }
  | { kind: "asking" }
  | { kind: "answering"; questions: StepQuestion[]; answers: Record<number, string> }
  | { kind: "rewriting"; questions: StepQuestion[]; answers: Record<number, string> }
  | { kind: "result"; suggestion: string; rationale: string }
  | { kind: "error"; message: string; recoverable: "asking" | "rewriting" };

const [phase, setPhase] = useState<Phase>({ kind: "idle" });
```

`previousValue` ref and `clearAssistant()` keep working as today; `clearAssistant` resets phase to idle.

### Flow

1. **Click ✦ Ask assistant** → `setPhase({ kind: "asking" })` → `await api.suggestStepQuestions(step.id, value)`
2. On success → `setPhase({ kind: "answering", questions, answers: {} })`
3. **User answers** by clicking option pills (sets `answers[i] = option`) or by typing into the "Other…" `TextField` (sets `answers[i] = freeText`). The answer for index `i` is whichever was last set.
4. **Click Generate suggestion** (enabled when every question has an answer) → `setPhase({ kind: "rewriting", questions, answers })` → `await api.suggestStep(step.id, value, qaPairs)`
5. On success → `setPhase({ kind: "result", suggestion, rationale })`
6. **Use this** (replaceable steps) sets `previousValue.current = value`, replaces the field, returns to idle.
7. **Skip — just suggest** (visible only in `answering`) jumps directly to the legacy path: `await api.suggestStep(step.id, value)` with no answers.

### New UI components (inside `OnboardingWizard.tsx`)

Small enough to live in the same file rather than spawning new files:

- `<QuestionRow>`: renders one question label, a horizontal flex of pill buttons, and an "Other…" pill that toggles an inline TextField below the row.
- `<PhaseBubble>`: switches between the 5 phase contents inside the existing accent-50 card. Avatar (Sparkles in accent-600 circle) is shared across phases.

### Pill button styling

Pills are slimmer than `OptionCard` — single line, ~32 px tall, accent-50 default with accent-300 border, accent-600 text; selected state inverts to accent-600 background with white text. Matches the existing "✦ Ask assistant" trigger button so the visual language is consistent.

### Behavior preserved

- Cmd/Ctrl+Enter still advances the wizard.
- Undo link below the input still appears after accepting a suggestion.
- Right-side "Why this step?" panel is untouched.
- Decision log writes are unchanged — the dialogue is ephemeral and not persisted.

## Error handling

| Failure | UX |
|---------|----|
| `suggestStepQuestions` fails | Bubble shows red message + Retry button. Retry re-runs the same call. |
| Model returns malformed JSON for questions | Same as above; backend returns a parse error. |
| `suggestStep` (with answers) fails | Bubble shows red message + Retry button. **Answers state is preserved** so the user doesn't have to re-pick. |
| LM Studio unreachable | "Could not reach LM Studio — check Settings." (existing message string) |
| No chat model configured | "Pick a chat model in Settings first." (existing message string) |

## Testing

Manual smoke test (no automated tests today for this surface):

1. Start app, create a new project, go through onboarding step 1 (`name`).
2. Type a partial name. Click **✦ Ask assistant**. Verify questions render as pill rows.
3. Pick option pills for each question; verify **Generate suggestion** stays disabled until all answered.
4. Click **Generate suggestion**; verify rewrite appears with rationale and **Use this**.
5. Click **Use this**; verify field is replaced and undo link appears.
6. Repeat for one long-text step (`domain`) and one choice step (`slr_type`); verify choice steps show advisory-only result (no Use this).
7. Click **Skip — just suggest** during `answering`; verify legacy one-shot path still works.
8. Force an error (kill LM Studio mid-flow); verify Retry preserves answers.

## Future work

- Multi-turn refinement: a "Refine further" button on the result that loops back to a follow-up question phase.
- Streaming the rewrite token-by-token.
- Persisting clarification Q&A to the decision log so reviewers can see the assistant's reasoning trail.
