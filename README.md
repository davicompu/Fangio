# Fangio

A desktop workshop for systematic literature reviews. Implements the SPAR-4-SLR
protocol (Paul et al. 2021) and PRISMA 2020 (Page et al. 2021), runs entirely on
your machine, talks only to a local LM Studio instance.

## Prerequisites

1. **Rust toolchain.** If you don't have it:
   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   source "$HOME/.cargo/env"
   ```
2. **Node 20+** (for the dev server and tooling).
3. **LM Studio** (optional for the foundation slice — required for any
   AI-assisted feature in later sessions). Download from
   [lmstudio.ai](https://lmstudio.ai) and start the local server from the
   Developer tab.

## Running the app

From this `app/` directory:

```sh
npm install        # one time
npm run tauri dev  # launches the desktop window
```

The first cargo build will take a few minutes (it has to compile sqlx, tauri,
reqwest with rustls, etc.). Subsequent runs are fast.

## What's in the foundation slice (Session 1)

- Project workspace model: pick a folder, Fangio creates `project.sqlite`,
  `data/`, `agent_reports/`, `imports/`, `pdfs/`, `drafts/`, `backups/`.
- Recent projects list, persisted at `~/.config/fangio/recent.json`.
- Onboarding wizard with the seven SPAR-4-SLR identification decisions
  (project name, SLR type, organising framework, domain, three RQs).
- "Why this step?" contextual docs beside every decision.
- Settings panel: LM Studio base URL, model dropdowns (chat + embedding),
  test connection, telemetry toggles, network allowlist viewer.
- Persistent status bar showing live LM Studio reachability, active models,
  latency, telemetry state, and project name.
- Local-only telemetry (decision saves and model calls).

## What's coming next

- **Session 2 — Assembling.** Search-strategy editor, RIS / BibTeX / WoS-tab /
  Scopus-CSV import pipeline, deduplication review, four-condition checkpoint.
- **Session 3 — Arranging.** Framework code definitions, batch coding tables,
  purification with criteria editor, PRISMA flow diagram with live counts.
- **Session 4 — Assessing.** PDF library, RAG over the corpus via LanceDB,
  draft generation, Pandoc export to .docx/.tex/.pdf.
- **Session 5 — Connectors + advisor.** Opt-in WoS / EBSCO / Scopus API
  connectors, embedded webview with advisor side panel.
- **Session 6 — Polish.** Backup/restore, multi-window safety, accessibility
  pass, end-to-end network-lockdown verification.

## Network policy

Fangio refuses to connect to anything that isn't `127.0.0.1` or `localhost`.
The `lmstudio.rs` client enforces this at the function level. Database
connectors compile in but are gated behind an explicit user opt-in and key
entry — none of which is exposed in the foundation slice.

To verify yourself: run the packaged app with `tcpdump` or Little Snitch and
confirm the only outbound traffic is to your LM Studio loopback port.
