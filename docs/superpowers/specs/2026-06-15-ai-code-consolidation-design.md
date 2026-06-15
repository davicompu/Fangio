# AI Code Consolidation — Design

**Date:** 2026-06-15
**Status:** Approved (design), implementing
**Area:** Stage 2 (Arranging) → Code-with-AI Raters, Phase 4 (after adjudication)

## Problem

Across a corpus, the model invents near-duplicate codes per article ("Structural
equation modelling" / "Structural equation modeling", "Higher education setting" /
"Business education"). The per-article adjudicator can't fix this — it only
compares the two raters within one article. A corpus-wide consolidation pass is
needed to collapse synonymous codes into canonical labels.

## Flow (Phase 4)

Runs as the final phase of an AI coding run, after adjudication, for any run
(dual or single rater), per dimension:

1. Gather all distinct applied codes corpus-wide for the dimension, each with its
   occurrence count and whether any occurrence is researcher-typed
   (`source = 'researcher'`).
2. Send the list to the reasoning model (`lmstudio.chat_model`): cluster codes
   that denote the **same concept** (semantic judgment, not string similarity);
   for each cluster return one **canonical label chosen from the list**.
   - Instruction: if a cluster contains a researcher-typed code, that label MUST
     be the canonical one.
3. **Auto-apply** (no approval step): for each cluster, for each non-canonical
   variant, rename matching `coding` rows to the canonical label.

## Guardrails

- Canonical must be one of the existing labels (never invents a new term).
- **Researcher-typed codes are never renamed** — they are canonical anchors only;
  AI variants merge into them. (Enforced in SQL: `WHERE source != 'researcher'`.)
- Per-dimension only.
- Model unreachable / unparsable / canonical-not-in-list → no-op for that
  dimension; codes untouched.

## Data model

- Add nullable column `consolidated_from TEXT` to `coding`: the original label a
  row was renamed from (powers display + undo).
- New table `ai_consolidation` (one row per variant→canonical merge): id,
  dimension, canonical_code, variant_code, count, run_id, created_at.

## Apply / undo

- Apply one merge: `UPDATE coding SET code = ?canonical, consolidated_from = ?variant
  WHERE dimension = ? AND code = ?variant AND source != 'researcher'`; insert an
  `ai_consolidation` row with the affected count.
- Undo one merge (`undo_consolidation(id)`): `UPDATE coding SET code =
  ?variant, consolidated_from = NULL WHERE dimension = ? AND code = ?canonical AND
  consolidated_from = ?variant`; delete the `ai_consolidation` row.

## UI

- `AiCodingResult` gains `consolidated: usize` and a `consolidations` list
  (`{id, dimension, canonical_code, variant_code, count}`).
- Progress phase `consolidating` ("Consolidating codes").
- The Code-with-AI modal "Done" panel shows a **Consolidated** count and a list
  grouped by canonical:
  `Higher education setting` *(was: Business education, Business higher education)* · **undo**
  — originals in muted parentheses, a small subtle **undo** per merge.
- Undo in the modal calls `undo_consolidation` and drops the line; the coding
  pane refreshes on modal close (existing `handleCodingComplete → onChanged`).
- `clear_all_coding` also clears `ai_consolidation` (coding rows are already
  wiped, so `consolidated_from` goes with them).

## Out of scope (YAGNI)

- No persistent per-chip "(was: X)" badge in the coding cards — the modal summary
  + the `ai_consolidation` record cover review and traceability for v1.
- No cross-dimension merging; no manual merge UI (separate future feature).
