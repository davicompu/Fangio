# AI Disagreement Adjudication — Design

**Date:** 2026-06-15
**Status:** Approved (design), pending implementation plan
**Area:** Stage 2 (Arranging) → Code-with-AI dual-rater coding

## Problem

Dual-rater AI coding produces a consensus set (themes both raters agree on) and a
review queue of leftover proposals. Even after fuzzy/case-insensitive matching
(`themes_match` in `lib.rs`), many leftover "disagreements" are not real conflicts —
they are the *same concept* worded with no shared tokens, which a string matcher
cannot catch. Examples:

- **Case 1 — synonymous context:** Rater 1 "Higher education settings" vs Rater 2
  "Academic settings". Either label works; they mean the same thing.
- **Case 2 — domain-knowledge equivalence:** Rater 1 "Structural equation modeling"
  vs Rater 2 "SmartPLS analysis". SmartPLS *is* a PLS-SEM tool, so both describe
  structural equation modeling.

A human currently has to resolve each of these by hand. An independent LLM with
domain knowledge can resolve the *easy* ones automatically, leaving only genuine
disagreements for the researcher.

This mirrors standard SLR practice: a third reviewer adjudicates ties between two
coders. We automate only the easy tier; the human keeps final say.

## Goals

- After both raters have coded **all** articles, an **independent adjudicator** model
  resolves semantically-equivalent disagreement pairs into single applied codes.
- Resolutions are **pre-applied (opt-out review)**: the user reviews only to *override*
  (Undo), not to Accept.
- Every machine-made merge is **logged and reversible** for SLR reproducibility.
- Genuine disagreements and single-rater complements are untouched, left for the human.

## Non-goals (YAGNI)

- No corpus-wide canonical theme registry (adjudication is per article + dimension).
- No new adjudicator model setting — reuses the configured reasoning/chat model.
- No resolution of genuinely different themes; no inventing brand-new labels.
- No re-adjudication of items the user has already Undone.

## Flow

The adjudicator is a **third phase** of a dual-rater AI coding run, after the existing
per-article rater-1 / rater-2 / consensus passes complete for **all** articles:

1. Rater 1 codes all articles.
2. Rater 2 codes all articles. Consensus + leftover proposals computed per article
   (unchanged from current behavior).
3. **Adjudication (new):** for each `(article, dimension)` that has leftover proposals
   from **both** raters, call the adjudicator model once with the two lists of
   `(label, rationale)`. It returns, for that group, the equivalent pairs and the
   single best canonical label for each pair.
4. For each equivalent pair: delete the two proposals, write one applied code
   (`source = "model-ai-adjudicated"`, `code = canonical label`), and record an
   `ai_adjudication` row.

Adjudication is **only run for dual-rater runs** (single-rater runs have no
disagreements).

## Scope / guardrails

- **Pairing unit:** same dimension of the same article only. Never merges across
  dimensions or across articles.
- **Equivalence only:** the model merges a pair only when it is confident the two
  labels denote the same concept. Uncertain or genuinely different → left as-is.
- **Canonical label** must be **one of the two existing labels** (the clearer / more
  standard one), or a trivial casing/normalization of one — never a newly invented term.
- **No counterpart:** a leftover theme only one rater raised, with no opposing theme in
  that dimension, is a *complement* and stays in the normal review queue. Adjudication
  resolves pairs only.

## Adjudicator model & prompt

- **Model:** the configured reasoning/chat model (`lmstudio.chat_model`), independent
  from the rater model(s). Temperature: `temperature.decisions` (low / deterministic).
- **Input per group:** the dimension name, Rater 1's leftover `(label, rationale)` list,
  Rater 2's leftover `(label, rationale)` list.
- **Instruction (conservative):** identify pairs that denote the *same* concept; for
  each, return the chosen canonical label (must be one of the inputs) and a one-line
  rationale. Leave anything uncertain unpaired.
- **Output:** strict JSON — a list of `{ rater1_label, rater2_label, canonical_label,
  rationale }`. Parsed defensively. Validation: `canonical_label` must match one of the
  two input labels **case-insensitively** (i.e. equal one of them ignoring casing); the
  stored code is then that input run through `canonical_theme()` for consistent casing.
  If `canonical_label` matches neither input, the pair is discarded and left for the
  human (the model may not invent a third label).

## Data model

New table `ai_adjudication` (one row per resolved pair), supporting Undo and audit:

| column | meaning |
|---|---|
| `id` | PK |
| `article_id` | article |
| `dimension` | dimension name |
| `canonical_code` | applied label |
| `coding_id` | FK to the `coding` row written for the resolution |
| `label1`, `rater1`, `rationale1` | Rater 1's original proposal (for restore) |
| `label2`, `rater2`, `rationale2` | Rater 2's original proposal (for restore) |
| `adjudicator_rationale` | model's one-line reasoning |
| `model_name` | adjudicator model |
| `created_at` | timestamp |

Decision log: each resolution also records a telemetry/decision entry
(`coding.adjudicated`) so the merge is traceable in the project history.

## Backend

- `lib.rs`: new adjudication phase in `run_ai_coding` (after the article loop, dual-rater
  only); a helper that calls the model per group and applies validated resolutions.
- New commands:
  - `undo_adjudication(id)` — delete the applied `coding` row, recreate the two original
    `ai_coding_proposal` rows from the stored labels, delete the `ai_adjudication` row.
  - `list_adjudications_for_articles(ids)` — for the review UI.
- `db.rs`: `ai_adjudication` CRUD; reuses `save_coding`, `save_ai_coding_proposal`,
  `delete_ai_coding_proposal`.
- Progress events: add an `coding-adjudicate` phase so the modal can show it.
- `AiCodingResult`: add `adjudicated: usize` count.

## Frontend

- `api.ts`: `undoAdjudication(id)`, `listAdjudicationsForArticles(ids)`.
- Review panel (the "AI SUGGESTIONS TO REVIEW" block): a muted **"Resolved by AI
  adjudicator"** section listing each resolution — canonical label, both original
  variants + rationales, the adjudicator's reasoning, and an **Undo** button. No Accept.
- `AiCodingModal`: third progress line and a result stat — *"N resolved by adjudicator."*

## Error handling

- Adjudicator unreachable / invalid JSON / canonical-label validation fails → that group
  (or pair) is a **no-op**: the proposals remain as normal disagreements for manual
  review. The phase never blocks the run and never loses data.
- Undo is idempotent: if the coding row is already gone, it still cleans up the
  adjudication row and restores proposals.

## Testing

- Backend unit tests for the JSON parsing + canonical-label validation (reject merges
  where the model returns a label that isn't one of the two inputs).
- Manual verification: re-run dual-rater coding on the Case 1 / Case 2 article; confirm
  both pairs land in "Resolved by AI adjudicator", Undo restores both originals to the
  review queue.
