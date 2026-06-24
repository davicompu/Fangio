# AI Coding: shared-codebook redesign + valid inter-rater agreement

Date: 2026-06-23
Status: approved (design) — implementing in phases

## Problem

AI coding runs two raters that **independently generate free-text codes** (1–3 noun
phrases per dimension, temperature 0.2). Two issues:

1. **Vocabulary divergence, not judgement divergence.** Raters produce
   complementary-but-differently-worded labels ("consumer trust" vs "trust
   formation"). `themes_match` catches casing/plural/subset variants but not
   semantic equivalents.
2. **Invalid agreement metric.** `compute_coding_agreement` scored presence/absence
   only over the *union* of codes the raters used per (article, dimension) — no
   "neither assigned" cells — so complementary coding becomes pure disagreement and
   Cohen's κ goes **negative ("worse than chance")** even when nothing conflicts.

## Design — mirror real SLR coding

Three AI roles:
- **Rater 1 / Rater 2** — the two coding models (+ optional profiles).
- **Moderator/Consolidator** — third AI (reasoning model) that owns the *canonical
  vocabulary*: builds the codebook, normalizes rater wording to canonical codes,
  adjudicates true disagreements. (Today's adjudication + consolidation passes are
  this role, applied at the wrong point.)

### Pass 1 — Construct the canonical codebook (corpus-level, per dimension)
- Seed with researcher-defined codes as **anchors** (canonical, never renamed away;
  tracked via existing `is_researcher`).
- Both raters propose candidate codes across the corpus (representative sample of
  ~30–40 articles on large corpora; "propose new code" in Pass 2 catches misses).
- Moderator consolidates pooled candidates (rater1 + rater2 + anchors) into one
  canonical codebook per dimension, collapsing wording variants.
- Written into the dimension's codes (the Organisation codebook). Fully automated —
  no blocking gate.

### Review checkpoint (between passes)
- Present the constructed codebook for the researcher to review/edit (merge, rename,
  delete). Default action is "proceed"; this does not stall automation, it's a
  sign-off on the coding scheme — the SLR norm.

### Pass 2 — Apply the codebook (per article)
- Both raters **select** applicable codes from the canonical codebook (closed-set);
  may propose a new code.
- Moderator canonicalizes each rater's output to a codebook entry; genuinely new
  codes are auto-added and flagged (no blocking).
- Constrained to the codebook → raters converge on vocabulary.

## Metric

- **Inter-rater agreement = Cohen's κ** on canonical codes. Valid now: fixed
  category set per dimension → real "neither assigned" cells. (Cronbach's α is for
  scale internal consistency — not applicable.)
- Also show plain **% agreement** and (interim) **average Jaccard set-overlap** for
  readability.

## Phasing

- **Pass 0 (now):** stop the misleading number — show Jaccard set-overlap + consensus
  rate instead of the invalid union-κ.
- **Phase 1:** codebook-construction pass (raters propose → moderator consolidates →
  write to Organisation, anchoring researcher codes).
- **Phase 1.5:** review checkpoint UI.
- **Phase 2:** closed-set coding against the codebook + canonicalization on apply.
- **Phase 3:** κ recomputed on canonical codes; sidebar card shows κ + % agreement.

## Open defaults (chosen unless changed)
- Pass 1 seeds from a representative sample (~30–40) on large corpora.
- Pass 2 allows new codes (moderator decides variant-vs-new; auto-add + flag).
