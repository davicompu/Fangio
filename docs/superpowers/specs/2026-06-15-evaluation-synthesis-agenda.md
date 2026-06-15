# Evaluation sub-stage: Synthesis report + guided/iterative Agenda

**Date:** 2026-06-15
**Status:** Approved, implementing
**Area:** Stage 3 (Assessing) → Evaluation (Synthesis, Agenda)

## Synthesis → generated, editable, cited report (feeds Results)

- Replace the open Q&A box with a **Generate synthesis** action. The command
  retrieves corpus chunks per framework dimension (embed "dimension: codes",
  top-k each) and asks the chat model to write a **structured, cited narrative
  organised by dimension**, grounded in those excerpts + the coded themes.
- Output is **editable markdown**, saved to `project.synthesis`, and the draft's
  **Results** section uses it when set (else the current placeholder).
- A **"Open chat with the literature"** button launches the existing RAG chat
  (onOpenRag) for ad-hoc questions — no duplicated Q&A box.
- `generate_synthesis` command + `suggest::generate_synthesis`.

## Agenda → clarify → generate → revise-by-prompt

- **Clarify:** three fixed free-text prompts before generating — intended
  contribution/audience, which gaps matter most, theories/methods to emphasize.
- **Generate:** `generate_agenda` takes optional `guidance` (the answers) +
  RQs + dimension findings.
- **Revise:** `generate_agenda` also takes `current` + `instruction`; when
  present it reworks the current agenda per the instruction. UI has a "Revise:
  …" box for unlimited rounds.
- Saved to `project.research_agenda` (unchanged storage).

## Backend

- Migration `0022_synthesis.sql`: add `project.synthesis TEXT`.
- ProjectRow + apply_decision: `project.synthesis`.
- `AgendaArgs { guidance?, current?, instruction? }`; extend
  `suggest::generate_research_agenda` with guidance/revise params.
- `draft.rs` §3 Results: use `project.synthesis` when set.

## Frontend

- New `SynthesisPanel` (replaces AskCorpusPanel in the Synthesis tab): Generate /
  edit / Save + "Open chat with the literature" (onOpenRag passed down).
- `AgendaPanel`: clarifying-question inputs, generate-with-guidance, and a
  revise-by-prompt box.
- `api.ts`: `generateSynthesis()`, `generateAgenda(args)`, ProjectRow.synthesis.
- `AssessingStage`: pass `onOpenRag` to SynthesisPanel.

## Out of scope
- AskCorpusPanel left in place but no longer used by the Synthesis tab.
- Clarifying questions are fixed (not AI-tailored) for predictability.
