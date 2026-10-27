# Changelog

All notable user-facing changes to Reeves.

## 0.4.3 — October 2026

### Added

- **Windows version (untested).** Reeves now builds for Windows 10/11 (64-bit)
  as a per-user setup program and an MSI package. It uses the native Windows
  title bar, opens and reveals files with the standard Windows apps, and shows
  Ctrl shortcuts. The installers aren't code-signed yet, so Windows shows a
  "Windows protected your PC" prompt: choose More info, then Run anyway. See
  docs/building-on-windows.md.
- **"Undo AI suggestion"** link under the search query after accepting the
  query agent's suggestion (⌘Z didn't work for this on macOS).

### Fixed

- One damaged entry in the recent-projects list no longer hides all of them.
- Glossary merge suggestions show the same paper counts as the glossary list.
- The glossary's A–Z index no longer gets cut off when the merges panel is open.
- AI screening decisions recorded without a reason now say so, and always
  show the decision.
- PDF passage previews start and end on whole words; search boxes no longer
  auto-capitalise.
- Removed outdated text on the Stage 1 checkpoint.

## 0.4.2 — October 2026

### Fixed

- Re-running a purification or journal-quality rule with a looser setting now
  clears the "Overridden" mark it had set on AI decisions (your own overrides
  are kept).
- Saving a synthesis or agenda you rewrote from scratch is no longer logged as
  an edited AI draft, and saving the same text twice isn't logged twice.
- A one-agent coding run in suggestion mode now says "Suggestions", not
  "Agreed suggestions".
- Renaming a code keeps its "opened" history; consolidated codes count opens
  under their earlier wording too.
- A glossary merge whose kept term disappeared after re-extracting is shown as
  "not in effect now" (it reapplies if the term comes back).
- If the backup before a database upgrade fails, Reeves explains why and what
  to do; the project is left unchanged.

## 0.4.1 — October 2026

### Fixed

- The decision report no longer says older screening decisions (made before
  the full log existed) were decided by the AI; it says this can't be
  determined.
- Switching embedding model without rebuilding the index could crash Clusters.
  Ask the corpus, Ask this article, Clusters and Synthesis now only use PDF
  text embedded with the current model.

## 0.4.0 — October 2026

### Added

- **AI-enhanced Library search** — choose *Traditional search* (exact words, as
  before) or *AI-enhanced search*, which ranks every paper by meaning across its
  title, abstract and indexed PDF text. Titles and abstracts not yet indexed are
  indexed first (you'll see progress); a slider sets how many results to show.
- **Glossary merges by meaning** — suggested pairs of terms that mean the same
  thing, with a similarity slider. Merge, undo, or mark "not the same"; nothing
  changes the concept map or overlap counts until you merge, and your decisions
  are kept when you re-extract.
- **Terms linked to every paper that mentions them** — each glossary term now
  counts every PDF paper in the review that uses the term or one of its
  aliases, not only the papers it was extracted from.
- **AI agent log** — the query agent (suggestion, reason, accept/undo/dismiss),
  codebook builder (proposals, how codes were grouped, your edits), synthesis
  and research agenda (each draft, and your save with how much you changed) are
  recorded with model, temperature, prompt fingerprint and seed. When you open
  coding suggestions, the codebook or a draft is recorded too. The decision
  report has a new "AI agents" section.
- **Setup assistant** entries now record temperature, prompt fingerprint and seed.

### Fixed

- Build index no longer reports a failure for the whole build when embedding
  titles and abstracts fails after the PDFs were indexed.
- Ask the Literature tools no longer write to the decision log.

## 0.3.3 — October 2026

### Added

- **Number of clusters** — choose how many clusters to build in Ask the
  Literature → Clusters, before the first build or afterwards next to Rebuild.
  "Automatic" keeps the previous behaviour. Each build is recorded in the
  decision log.
- **Glossary A–Z index** — jump between letters from the index on the right.

## 0.3.2 — October 2026

### Added

- **Titles and abstracts are embedded** when you build the index in Ask the
  Literature, using your embedding model, so they can be searched by meaning.
  Only changed titles/abstracts are re-embedded on later builds.

## 0.3.1 — October 2026

### Added

- **Setup assistant log** — every use of "Ask assistant" during setup (and when
  editing the setup answers later) is recorded in the decision log: the text in
  the field, the earlier answers sent as context, the clarifying questions and
  the researcher's answers (or "Skip — just suggest"), the suggestion and its
  reason, the model that answered, and whether the researcher used it, undid it
  or kept their own answer. The decision report lists these in a new section.

## 0.3.0 — October 2026

### Added

- **Review status for AI screening decisions** — every AI decision shows whether
  it was applied automatically, opened and left unchanged, overridden, or is
  pending. Opening a decision is logged with the time and who opened it.
- **Reminders before moving on** — before Stage 3 or an export, Reeves reminds
  you about AI decisions that were applied automatically or are still pending.
  You can always continue.
- **Agreed codes as suggestions** — a setting to show codes both coding agents
  agree on as suggestions instead of applying them, with "Accept all agreed".
- **Confidence on coding suggestions** — the model's own estimate, not a
  measured probability.
- **Merging and consolidating models** — pick their models, with a warning when
  one is also a coding agent. Their instructions, and the synthesis agent's,
  are editable in Settings.
- **Optional seed** setting, recorded in the decision report.

### Changed

- **Complete screening log** — every AI screening outcome is recorded, including
  full-text includes, unsure results, purification and model errors, with the
  model used, a prompt fingerprint and the temperature.
- Undoing or resetting a screening decision now keeps the previous AI reason
  and confidence in the log.
- Agreed codes keep both coding agents' reasons; accepting a suggestion is logged.
- The third coding agent is now called the **merging agent**; undoing a merge is logged.
- The decision report lists the full AI screening log, decisions applied
  automatically, and model, prompt fingerprint and temperature for every AI decision.
- Wording now says plainly what goes online (reference lookups, open-access PDF
  searches, the database browser, and any remote model server).
- Projects are backed up automatically before a database upgrade.

## 0.2.0 — June 2026

### Added

- **Concept map** — a new tab in Ask the Literature that draws the corpus as an
  interactive graph: glossary concepts linked to the articles that mention them.
- **Glossary** — Reeves now extracts domain terms and definitions from your
  indexed PDFs. Variants like "Critical Thinking (CT)" and "CT" are recognised
  as one concept and merged automatically.
- **Clusters** — K-means grouping of the corpus by embedding similarity, with
  LLM-labelled themes.
- **Background tasks** — cluster builds, glossary extraction, and RAG indexing
  keep running (and keep reporting progress) when you navigate to another view.
- **Per-feature sampling controls** — separate temperature knobs for screening
  decisions, suggestions, and Q&A in Settings → Models.
- **PRISMA diagram export** — download the flow diagram as SVG or high-resolution PNG.

### Changed

- Chat answers and AI suggestions are now sanitised before rendering, hardening
  the app against malicious content embedded in imported PDFs.

### Removed

- The remote companion (phone access over Tailscale) has been removed to keep
  Reeves fully local. It is preserved on the `with-side-companion` branch.

## 0.1.0 — April 2026

- Initial release: SPAR-4-SLR staged workflow, PRISMA 2020 tracking, local
  LM Studio integration, AI-assisted screening and coding, RAG chat over your
  corpus, dedup review, BibTeX/CSV export, and per-project SQLite storage.
