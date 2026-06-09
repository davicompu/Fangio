# Changelog

All notable user-facing changes to Reeves.

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
