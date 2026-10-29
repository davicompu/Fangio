# Reeves

A desktop workshop for systematic literature reviews, following the SPAR-4-SLR
protocol (Paul et al. 2021) and PRISMA 2020 (Page et al. 2021). Your project
lives in a folder on your computer. Some features go online; see
[What goes over the network](#what-goes-over-the-network).

**[Reeves in pictures](docs/reeves-in-pictures.md)** walks through a real review
screen by screen, for anyone who'd like to see the app before installing it.

Tested on macOS with LM Studio. A Windows version builds from the same code
(see [Building on Windows](docs/building-on-windows.md)) but hasn't been tested
yet. Ollama should work but is untested. Linux is untested.

## Download and install

Ready-built installers for version 0.4.3 are in the [`downloads`](downloads)
folder. Open the file on GitHub and click the download button (the arrow at the
top right of the file view).

| Platform | File |
| --- | --- |
| macOS, Apple Silicon (M1 or newer) | [Reeves_0.4.3_aarch64.dmg](downloads/Reeves_0.4.3_aarch64.dmg) |
| Windows 10 or 11, 64-bit | [Reeves_0.4.3_x64-setup.exe](downloads/Reeves_0.4.3_x64-setup.exe) |

There is no build for Intel Macs or Linux yet; build from source instead (see
below). The installers aren't signed with an Apple or Microsoft developer
certificate, so both systems will warn you the first time.

### macOS

1. Double-click `Reeves_0.4.3_aarch64.dmg` and drag **Reeves** onto the
   **Applications** folder.
2. Open Reeves from Applications. macOS will refuse, saying it can't verify the
   developer or that the app is damaged. Click **Done** (not Move to Trash).
3. Open **System Settings → Privacy & Security**, scroll down to the message
   about Reeves, and click **Open Anyway**. Confirm with your password.

If macOS says the app "is damaged" and there's no Open Anyway button, run this
once in Terminal and then open Reeves again:

```sh
xattr -dr com.apple.quarantine /Applications/Reeves.app
```

### Windows

1. Double-click `Reeves_0.4.3_x64-setup.exe`. It installs Reeves for your user
   account; no administrator rights are needed.
2. If Windows shows **"Windows protected your PC"**, click **More info**, then
   **Run anyway**.
3. The installer downloads Microsoft's WebView2 component if the PC doesn't
   have it already (Windows 11 and recent Windows 10 do).

Reeves then appears in the Start menu. The Windows build hasn't been tested
yet; please report anything that doesn't work.

### AI features

The app runs without a model server, but the AI-assisted features need one; see
step 3 of the prerequisites below.

## Building from source

### Prerequisites

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

### Running the app

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
