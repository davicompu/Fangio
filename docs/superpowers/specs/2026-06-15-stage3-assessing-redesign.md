# Stage 3 (Assessing) Redesign — SPAR-4-SLR Evaluation + Reporting

**Date:** 2026-06-15
**Status:** Approved (full redesign), implementing
**Area:** Stage 3 (Assessing)

## Rationale

SPAR-4-SLR Stage 3 "Assessing" has two sub-stages (Paul et al., 2021):
- **3a Evaluation** — analysis method applied to the synthesized corpus + the
  research **agenda proposal** (future directions from the gaps).
- **3b Reporting** — reporting conventions (figures/tables), review
  **limitations**, and **sources of support** (funding). PRISMA 2020 is the
  reporting standard (flow + 27-item checklist).

Current Stage 3 is four flat tool tabs (PDF library, Ask the corpus, Draft,
Export) and omits the protocol deliverables (agenda, analysis method,
limitations, funding) and doesn't surface PRISMA in-stage.

## New structure (single tab strip, ordered by sub-stage)

**3a · Evaluation**
1. **Corpus** — existing PdfLibraryPanel (full texts for assessment).
2. **Findings** — NEW. Coding frequencies per framework dimension (from Stage 2
   codings) as the evidence/findings table, plus an **Analysis method** selector
   (content / thematic / framework-based TCCM / framework-based ADO /
   bibliometric / other) saved to the project.
3. **Synthesis** — existing AskCorpusPanel (cited passages).
4. **Agenda** — NEW. Research-agenda editor; "Generate with AI" drafts future
   directions from the RQs + dimension findings; editable; saved to the project.

**3b · Reporting**
5. **PRISMA** — existing PrismaPanel surfaced here (flow diagram + checklist).
6. **Draft** — existing DraftPanel, preceded by **Limitations** and **Sources of
   support / funding** fields; the generated manuscript now includes agenda,
   limitations, funding, and the analysis method.
7. **Export** — existing ExportPanel.

## Backend

- Migration `0021_assessing_fields.sql`: add nullable TEXT columns to `project`:
  `analysis_method`, `research_agenda`, `review_limitations`, `sources_of_support`.
- `Project` struct (db.rs) + `ProjectRow` (api.ts): add the four fields.
- `apply_decision`: map `project.analysis_method`, `project.research_agenda`,
  `project.review_limitations`, `project.sources_of_support` onto those columns.
- `suggest::generate_research_agenda(...)` + `generate_agenda` command: prompt
  the chat model with domain, RQs, framework, and the dimension→code frequency
  table; return a markdown agenda. Unreachable model → error surfaced in UI.
- `draft.rs`: weave analysis method into the Method section, the research agenda
  into a "Future research agenda" section, review limitations into Limitations,
  and sources of support into an "Acknowledgements / funding" line — each only
  when the project field is set (else keep current defaults).

## Frontend

- `AssessingStage`: 7-tab strip ordered Evaluation→Reporting; `tabInfo` per tab
  notes its sub-stage (3a/3b); right-rail "About this step" copy.
- NEW `FindingsPanel`: frequencies via `listCodingBatch` + `listFrameworkDimensions`;
  analysis-method `<Select>` persists via `saveDecision("project.analysis_method")`.
- NEW `AgendaPanel`: textarea bound to `project.research_agenda`; "Generate with
  AI" calls `generateAgenda`; Save persists via `saveDecision`.
- `DraftPanel`: add Limitations + Sources-of-support text fields (persist via
  `saveDecision`) above the existing generate/edit controls.
- `api.ts`: `generateAgenda()`; ProjectRow fields.
- `StageHub`: keep substages label as ["Evaluation", "Reporting"] (drop the
  stray "Draft" — Draft is a tool under Reporting).

## Out of scope (YAGNI)

- No new analysis engines (bibliometric charts etc.) — analysis method is a
  recorded choice that frames the manuscript, not an automated analysis.
- No change to PRISMA generation (reused as-is).
