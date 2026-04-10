# In-App Database Browser — Design Spec

**Date:** 2026-04-09
**Stage affected:** Stage 1 · Assembling → Acquisition tab + Settings
**Feature:** Embedded browser panel for running database searches and auto-importing results

---

## Overview

Add an in-app browser to the Acquisition tab so users can navigate to their selected academic databases, run their boolean query, and download references — all without leaving Fangio. Downloaded reference files (RIS, BibTeX, WoS, Scopus CSV) are intercepted by Tauri and imported automatically with a one-click prompt.

---

## Architecture

### Components affected

| Action | Path | Purpose |
|--------|------|---------|
| Modify | `src/views/assembling/AcquisitionForm.tsx` | Add "Run searches" row, browser panel state, import banner |
| Modify | `src/views/Settings.tsx` | Add "Database access" section with URL fields per database |
| Create | `src-tauri/src/browser.rs` | Tauri commands for child webview lifecycle + download interception |
| Modify | `src-tauri/src/lib.rs` | Register new Tauri commands |
| Modify | `src/lib/api.ts` | Add typed wrappers for browser commands |
| Create | `src-tauri/migrations/0005_database_urls.sql` | Seed default Google Scholar URL in settings |

### No new database tables needed

Database URLs are stored in the existing `settings` table using the pattern `database.<id>.url` (e.g. `database.scopus.url`). The existing `api.setSetting` / `api.getSettings` infrastructure handles reads and writes.

---

## Settings — Database Access Section

A new **"Database access"** section in the Settings panel lists all 8 supported databases, each with a URL text field.

**Databases and their keys:**

| Database | Settings key | Default value |
|----------|-------------|---------------|
| Scopus | `database.scopus.url` | `""` |
| Web of Science | `database.wos.url` | `""` |
| EBSCO Business Source Complete | `database.ebsco-bsc.url` | `""` |
| PubMed | `database.pubmed.url` | `""` |
| IEEE Xplore | `database.ieee.url` | `""` |
| ACM Digital Library | `database.acm.url` | `""` |
| Google Scholar | `database.scholar.url` | `"https://scholar.google.com"` |
| ScienceDirect | `database.sciencedirect.url` | `""` |

**Seeded in migration:** Google Scholar's default URL is seeded in a new migration so new projects get it pre-filled. All others default to empty string.

**UI:** Each row shows the database name and a `TextField` for the URL. No save button — changes write immediately via `api.setSetting` on blur (same pattern used elsewhere in Settings).

---

## Acquisition Tab — Run Searches Section

### Database checkboxes (unchanged)

All 8 databases remain selectable regardless of whether a URL is configured. Selection tracks which databases the researcher intends to search (for record-keeping). Databases without a configured URL show a small inline note: "Add URL in Settings to enable in-app search."

### "Run searches" row

A new sub-section below the database grid, only rendered when at least one selected database has a configured URL. Shows one button per (selected ∧ URL-configured) database:

```
[ Scopus ]  [ Web of Science ]  [ Google Scholar ]
```

If the user has selected databases but none have URLs: a prompt reads "No databases configured yet — add at least one URL in Settings to search here" with a link-style button opening Settings.

If no databases are selected at all: the row is hidden entirely.

### Layout change when browser is open

The tab's grid changes from `grid-cols-[1fr_320px]` to `grid-cols-[1fr_minmax(0,55%)]`. The right column is occupied by the browser panel. The advisor card is hidden while the browser is open. The form remains fully visible and editable in the left column.

A close button (`X`) in the browser panel header restores the advisor card and resets the grid.

Only one browser panel can be open at a time. Clicking a different database button closes the current panel and opens a new one at the new URL.

---

## Browser Panel — Tauri Child Webview

### How it opens

1. React renders a placeholder `<div ref={panelRef}>` that occupies the right column
2. After layout settles (`requestAnimationFrame`), `getBoundingClientRect()` measures the div's pixel position and size
3. `api.openBrowserPanel(url, x, y, width, height)` is called with those values
4. Rust creates a child `Webview` via `tauri::WebviewBuilder` at those exact window coordinates, with download interception enabled

### Resize handling

A `ResizeObserver` watches the placeholder div. On size change, `api.resizeBrowserPanel(x, y, width, height)` repositions the native webview to match.

### Closing

`api.closeBrowserPanel()` destroys the child webview. React restores the advisor card.

### Rust commands

```rust
// src-tauri/src/browser.rs

#[tauri::command]
pub async fn open_browser_panel(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String>

#[tauri::command]
pub async fn resize_browser_panel(
    state: tauri::State<'_, AppState>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String>

#[tauri::command]
pub async fn close_browser_panel(
    state: tauri::State<'_, AppState>,
) -> Result<(), String>
```

The `AppState` holds an `Option<(tauri::Webview, String)>` — the active webview and the `source_database` id it was opened for (e.g. `"scopus"`). This is needed so the `on_download` handler can include the correct `source_database` in the emitted event. `open_browser_panel` closes any existing panel before creating a new one.

### Download interception

The child webview is built with an `on_download` handler:

- Files with extensions `.ris`, `.bib`, `.txt`, `.csv` → saved to a Fangio temp directory (`{workspace}/.tmp/downloads/`); a `browser:download-ready` event is emitted to the frontend with `{ path, filename, source_database }`
- All other file types → allowed to download normally to the system Downloads folder

---

## Import Banner (Left Column)

When the frontend receives a `browser:download-ready` event, a sticky banner appears at the top of the form column:

```
export.ris downloaded from Scopus  [Import]  [Dismiss]
```

- **Import**: calls `api.commitImport(path, sourceDatabase)` using the existing import pipeline. Banner disappears on success.
- **Dismiss**: removes the banner without importing. The temp file remains in `{workspace}/.tmp/downloads/` and is cleaned up the next time `close_browser_panel` is called or when the project is opened.
- If parsing fails: the banner shows the error message inline (same error format as the Import tab).
- Multiple downloads stack as multiple banners.

The banner is a React component rendered in the left column above the "Search mechanisms" section, so it is never obscured by the native webview layer.

---

## Data Flow

```
User clicks "Scopus" button in Run searches row
  → React measures placeholder div rect
  → api.openBrowserPanel("https://...", x, y, w, h)
    → Rust: creates child Webview at those coords with on_download handler
  → Scopus advanced search opens in-panel

User runs search, clicks "Export as RIS"
  → on_download fires
  → file saved to {workspace}/.tmp/downloads/export.ris
  → event emitted: browser:download-ready { path, filename: "export.ris", source_database: "scopus" }
  → React: import banner appears in left column

User clicks Import in banner
  → api.commitImport(path, "scopus")
  → existing import pipeline runs
  → banner disappears, assembling stats update

User clicks X in browser panel
  → api.closeBrowserPanel()
  → Rust destroys child webview
  → React restores advisor card, resets grid layout
```

---

## Error Handling

- **No databases configured:** prompt in "Run searches" row directing user to Settings
- **URL invalid / site unreachable:** the webview shows its own browser error page (standard network error UI)
- **Download parse failure:** banner shows error text, user can dismiss
- **Panel creation failure:** `open_browser_panel` returns an error string; React shows an inline error in the right column instead of the panel

---

## Out of Scope

- Pre-filling the boolean query into the database search field (each database has a different URL parameter scheme; the query is visible in the left column for copy-paste)
- Streaming/scraping search results directly (users export files and import them)
- Browser history or bookmarks within the panel
- Multiple simultaneous panels
