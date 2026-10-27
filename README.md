# Reeves

A desktop workshop for systematic literature reviews, following the SPAR-4-SLR
protocol (Paul et al. 2021) and PRISMA 2020 (Page et al. 2021). Your project
lives in a folder on your computer. Some features go online; see
[What goes over the network](#what-goes-over-the-network).

Tested on macOS with LM Studio. A Windows version builds from the same code
(see [Building on Windows](docs/building-on-windows.md)) but hasn't been tested
yet. Ollama should work but is untested. Linux is untested.

## Prerequisites

1. **Rust toolchain.** If you don't have it:
   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   source "$HOME/.cargo/env"
   ```
2. **Node 20+** (for the dev server and tooling).
3. **An AI model server** (optional, needed for AI-assisted features). Any
   OpenAI-compatible server works. Tested with LM Studio on macOS; download it
   from [lmstudio.ai](https://lmstudio.ai) and start the local server from the
   Developer tab. Ollama should work but is untested.

## Running the app

On Windows, follow [Building on Windows](docs/building-on-windows.md) instead.

From the repository root:

```sh
npm install        # one time
npm run tauri dev  # launches the desktop window
```

The first cargo build will take a few minutes (it has to compile sqlx, tauri,
reqwest with rustls, etc.). Subsequent runs are fast.

## What it does

- **Projects.** Pick a folder; Reeves creates `project.sqlite`, `data/`,
  `agent_reports/`, `imports/`, `pdfs/`, `drafts/` and `backups/` in it. Recent
  projects are listed from `recent.json` in the `carta` folder of your OS config
  directory (on macOS, `~/Library/Application Support/carta/recent.json`; on
  Windows, `%APPDATA%\carta\recent.json`).
- **Assembling.** Identification, acquisition, importing, journals, library,
  deduplication and a checkpoint.
- **Arranging.** Organisation by your framework, purification criteria,
  title and abstract screening, full-text screening, coding, PRISMA flow, and
  a review-and-finish step.
- **Assessing.** Corpus, findings, synthesis, agenda, PRISMA, draft and export.
- **Settings.** Model server and models, AI rater prompts, temperatures and an
  optional seed, privacy, backups.

## What goes over the network

- **Crossref** (`api.crossref.org`): the DOI, when you look up or enrich a reference.
- **Unpaywall** (`api.unpaywall.org`) and the publisher sites it points to: the DOI and Reeves' contact address, when you search for open-access PDFs.
- **Database browser and "Find online"**: the pages you open, as in any browser.
- **Your AI model server**: titles, abstracts, full-text excerpts, codes and prompts. With LM Studio on your own computer nothing leaves it. A remote server address sends this content to that server.
- **Stays local**: the project database, PDFs, drafts, backups, and usage statistics (telemetry). Reeves sends no usage data anywhere.
