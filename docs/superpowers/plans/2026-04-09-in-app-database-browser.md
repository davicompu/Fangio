# In-App Database Browser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an embedded browser panel to the Acquisition tab so users can open selected databases, run their boolean query, and have downloaded reference files (.ris/.bib/.txt/.csv) automatically imported into Fangio.

**Architecture:** A new Tauri child webview (`browser.rs`) is created at the exact pixel position of a React placeholder `<div>` in the right column of the Acquisition tab. Database URLs are stored in the existing `settings` table and managed in a new "Database access" section in Settings. A `ResizeObserver` keeps the webview aligned as the window resizes. Download interception saves reference files to `{workspace}/.tmp/downloads/` and emits a `browser:download-ready` event; the frontend renders import banners in the left column.

**Tech Stack:** React/TypeScript, Tauri v2 (`tauri::WebviewBuilder`, `window.add_child`, `DownloadEvent`), Rust `std::sync::Mutex`, `@tauri-apps/api/event` (listen), Tailwind CSS, Framer Motion

---

## File Map

| Action | Path | Purpose |
|--------|------|---------|
| Create | `src-tauri/migrations/0005_database_urls.sql` | Seed Google Scholar URL default |
| Modify | `src/lib/useAppState.ts` | Add 8 `database.<id>.url` keys to `AppSettings` |
| Modify | `src/views/Settings.tsx` | Add "Database access" section |
| Modify | `src/App.tsx` | Pass `settings` + `onOpenSettings` to `AssemblingStage` |
| Modify | `src/views/assembling/AssemblingStage.tsx` | Accept + forward `settings` + `onOpenSettings` |
| Modify | `src/views/assembling/AcquisitionForm.tsx` | Add settings props, "Run searches" row, browser panel, import banners |
| Create | `src-tauri/src/browser.rs` | `BrowserState`, child webview lifecycle, download interception |
| Modify | `src-tauri/src/lib.rs` | Register `mod browser`, manage `BrowserState`, register 3 new commands, clean temp dir on open |
| Modify | `src/lib/api.ts` | Add `openBrowserPanel`, `resizeBrowserPanel`, `closeBrowserPanel` methods |

---

## Task 1: Migration — seed Google Scholar default URL

**Files:**
- Create: `src-tauri/migrations/0005_database_urls.sql`

- [ ] **Step 1: Write the migration**

Create `src-tauri/migrations/0005_database_urls.sql`:
```sql
-- Seed database access URLs. Google Scholar is free so it gets a default.
-- All others are left empty; users fill in their institutional access URLs.
INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES
    ('database.scopus.url',        '""', datetime('now')),
    ('database.wos.url',           '""', datetime('now')),
    ('database.ebsco-bsc.url',     '""', datetime('now')),
    ('database.pubmed.url',        '""', datetime('now')),
    ('database.ieee.url',          '""', datetime('now')),
    ('database.acm.url',           '""', datetime('now')),
    ('database.scholar.url',       '"https://scholar.google.com"', datetime('now')),
    ('database.sciencedirect.url', '""', datetime('now'));
```

- [ ] **Step 2: Verify Rust compiles and migration is picked up**

```bash
cd /Users/david/Documents/GitHub/Fangio && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```

Expected: `Finished` with no errors (sqlx migrate! macro discovers migrations by filename order).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/migrations/0005_database_urls.sql
git commit -m "feat(browser): seed database URL defaults in settings"
```

---

## Task 2: AppSettings type + Settings UI — Database access section

**Files:**
- Modify: `src/lib/useAppState.ts` (lines 4–18)
- Modify: `src/views/Settings.tsx`

- [ ] **Step 1: Add 8 database URL keys to `AppSettings` and `DEFAULTS`**

In `src/lib/useAppState.ts`, find:
```ts
export interface AppSettings {
  "lmstudio.base_url": string;
  "lmstudio.chat_model": string;
  "lmstudio.embed_model": string;
  "telemetry.enabled": boolean;
  "telemetry.log_reasoning": boolean;
}

const DEFAULTS: AppSettings = {
  "lmstudio.base_url": "http://127.0.0.1:1234/v1",
  "lmstudio.chat_model": "",
  "lmstudio.embed_model": "",
  "telemetry.enabled": true,
  "telemetry.log_reasoning": false,
};
```

Replace with:
```ts
export interface AppSettings {
  "lmstudio.base_url": string;
  "lmstudio.chat_model": string;
  "lmstudio.embed_model": string;
  "telemetry.enabled": boolean;
  "telemetry.log_reasoning": boolean;
  "database.scopus.url": string;
  "database.wos.url": string;
  "database.ebsco-bsc.url": string;
  "database.pubmed.url": string;
  "database.ieee.url": string;
  "database.acm.url": string;
  "database.scholar.url": string;
  "database.sciencedirect.url": string;
}

const DEFAULTS: AppSettings = {
  "lmstudio.base_url": "http://127.0.0.1:1234/v1",
  "lmstudio.chat_model": "",
  "lmstudio.embed_model": "",
  "telemetry.enabled": true,
  "telemetry.log_reasoning": false,
  "database.scopus.url": "",
  "database.wos.url": "",
  "database.ebsco-bsc.url": "",
  "database.pubmed.url": "",
  "database.ieee.url": "",
  "database.acm.url": "",
  "database.scholar.url": "https://scholar.google.com",
  "database.sciencedirect.url": "",
};
```

- [ ] **Step 2: Add "Database access" section to `Settings.tsx`**

In `src/views/Settings.tsx`, add `Globe` to the lucide-react imports:
```tsx
import {
  Camera,
  CheckCircle2,
  CircleAlert,
  Copy,
  Cpu,
  Globe,
  HardDrive,
  Loader2,
  Lock,
  Radio,
  ShieldCheck,
  X,
} from "lucide-react";
```

Then find the closing `{/* Network sovereignty */}` section and insert a new section **before** it:

```tsx
              {/* Database access */}
              <section>
                <SectionHeader
                  icon={<Globe size={14} strokeWidth={1.75} />}
                  title="Database access"
                  description="Paste the advanced-search URL for each database you have institutional access to. Fangio opens these in an in-app browser so you can run your boolean query without leaving the app. Leave empty if you don't have access."
                />
                <div className="mt-5 space-y-3">
                  {([
                    { key: "database.scopus.url" as const, label: "Scopus" },
                    { key: "database.wos.url" as const, label: "Web of Science" },
                    { key: "database.ebsco-bsc.url" as const, label: "EBSCO Business Source Complete" },
                    { key: "database.pubmed.url" as const, label: "PubMed" },
                    { key: "database.ieee.url" as const, label: "IEEE Xplore" },
                    { key: "database.acm.url" as const, label: "ACM Digital Library" },
                    { key: "database.scholar.url" as const, label: "Google Scholar" },
                    { key: "database.sciencedirect.url" as const, label: "ScienceDirect" },
                  ] as { key: keyof AppSettings; label: string }[]).map(({ key, label }) => (
                    <TextField
                      key={key}
                      label={label}
                      value={settings[key] as string}
                      onChange={(e) => onUpdate(key, e.target.value)}
                      onBlur={(e) => onUpdate(key, e.target.value)}
                      placeholder="https://..."
                      mono
                    />
                  ))}
                </div>
              </section>
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio && npx tsc --noEmit 2>&1 | head -20
```

Expected: no output (no errors).

- [ ] **Step 4: Commit**

```bash
git add src/lib/useAppState.ts src/views/Settings.tsx
git commit -m "feat(browser): add database access URL settings"
```

---

## Task 3: Prop chain — pass `settings` + `onOpenSettings` to `AcquisitionForm`

**Files:**
- Modify: `src/App.tsx` (line 90)
- Modify: `src/views/assembling/AssemblingStage.tsx` (lines 20–23, 65, 147)
- Modify: `src/views/assembling/AcquisitionForm.tsx` (lines 10–13, 26)

- [ ] **Step 1: Add `settings` and `onOpenSettings` to `AssemblingStageProps`**

In `src/views/assembling/AssemblingStage.tsx`, find:
```tsx
interface AssemblingStageProps {
  project: ProjectRow;
  onLeaveStage: () => void;
}
```

Replace with:
```tsx
interface AssemblingStageProps {
  project: ProjectRow;
  settings: AppSettings;
  onLeaveStage: () => void;
  onOpenSettings: () => void;
}
```

Also add the `AppSettings` import — find:
```tsx
import { api, type AssemblingState, type ProjectRow } from "../../lib/api";
```

Replace with:
```tsx
import { api, type AssemblingState, type ProjectRow } from "../../lib/api";
import type { AppSettings } from "../../lib/useAppState";
```

- [ ] **Step 2: Destructure and forward the new props in `AssemblingStage`**

Find:
```tsx
export function AssemblingStage({ project, onLeaveStage }: AssemblingStageProps) {
```

Replace with:
```tsx
export function AssemblingStage({ project, settings, onLeaveStage, onOpenSettings }: AssemblingStageProps) {
```

Find the `AcquisitionForm` usage:
```tsx
            {tab === "acquisition" && (
              <AcquisitionForm project={project} onSaved={onChanged} />
            )}
```

Replace with:
```tsx
            {tab === "acquisition" && (
              <AcquisitionForm
                project={project}
                settings={settings}
                onSaved={onChanged}
                onOpenSettings={onOpenSettings}
              />
            )}
```

- [ ] **Step 3: Add `settings` and `onOpenSettings` to `AcquisitionForm` props**

In `src/views/assembling/AcquisitionForm.tsx`, find:
```tsx
interface Props {
  project: ProjectRow;
  onSaved: () => void;
}
```

Replace with:
```tsx
import type { AppSettings } from "../../lib/useAppState";

interface Props {
  project: ProjectRow;
  settings: AppSettings;
  onSaved: () => void;
  onOpenSettings: () => void;
}
```

And update the function signature — find:
```tsx
export function AcquisitionForm({ project, onSaved }: Props) {
```

Replace with:
```tsx
export function AcquisitionForm({ project, settings, onSaved, onOpenSettings }: Props) {
```

Note: add the `AppSettings` import **before** the `interface Props` block, not inside it. The import line goes near the top of the file after the existing imports.

- [ ] **Step 4: Pass the new props from `App.tsx`**

In `src/App.tsx`, find:
```tsx
            <AssemblingStage project={project} onLeaveStage={() => setRoute("hub")} />
```

Replace with:
```tsx
            <AssemblingStage
              project={project}
              settings={settings}
              onLeaveStage={() => setRoute("hub")}
              onOpenSettings={() => setSettingsOpen(true)}
            />
```

- [ ] **Step 5: Verify TypeScript compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio && npx tsc --noEmit 2>&1 | head -20
```

Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add src/App.tsx src/views/assembling/AssemblingStage.tsx src/views/assembling/AcquisitionForm.tsx
git commit -m "feat(browser): thread settings + onOpenSettings to AcquisitionForm"
```

---

## Task 4: "Run searches" row in `AcquisitionForm`

**Files:**
- Modify: `src/views/assembling/AcquisitionForm.tsx`

Context: The DATABASES array at the top of the file contains `id` values like `"scopus"`, `"wos"`, `"ebsco-bsc"`, etc. The `settings["database.<id>.url"]` pattern gives the configured URL for each. Add browser state variables and the "Run searches" section.

- [ ] **Step 1: Add browser state variables and imports**

In `src/views/assembling/AcquisitionForm.tsx`, find the existing imports and add `Globe` to the lucide-react import:
```tsx
import { Calendar, Database, Globe, Lightbulb, Save, Search, Sparkles } from "lucide-react";
```

Then find:
```tsx
  const [saveError, setSaveError] = useState<string | null>(null);
```

Add immediately after:
```tsx
  const [browserDb, setBrowserDb] = useState<string | null>(null);
  const [browserUrl, setBrowserUrl] = useState("");
```

- [ ] **Step 2: Add "Run searches" section JSX**

In `src/views/assembling/AcquisitionForm.tsx`, find the closing `</section>` of the Boolean query section (the last `</section>` before the Save button `<div>`). After that closing `</section>`, insert:

```tsx
        {/* Run searches — only shown when databases are selected */}
        {databases.size > 0 && (() => {
          const searchable = DATABASES.filter(
            (d) => databases.has(d.id) && settings[`database.${d.id}.url` as keyof typeof settings],
          );
          const selectedNoUrl = DATABASES.filter(
            (d) => databases.has(d.id) && !settings[`database.${d.id}.url` as keyof typeof settings],
          );
          return (
            <section>
              <SectionHeader
                icon={<Globe size={14} strokeWidth={1.75} />}
                title="Run searches"
                description="Open a database in-app to run your query and import results directly."
              />
              <div className="mt-4 flex flex-wrap items-center gap-2">
                {searchable.map((d) => (
                  <button
                    key={d.id}
                    type="button"
                    onClick={() => {
                      setBrowserDb(d.id);
                      setBrowserUrl(
                        settings[`database.${d.id}.url` as keyof typeof settings] as string,
                      );
                    }}
                    className={
                      "px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium transition-paper " +
                      (browserDb === d.id
                        ? "bg-[var(--color-ink-700)] text-white"
                        : "bg-[var(--color-paper-100)] border border-[var(--color-paper-300)] text-[var(--color-ink-800)] hover:border-[var(--color-paper-500)]")
                    }
                  >
                    {d.label}
                  </button>
                ))}
                {searchable.length === 0 && (
                  <button
                    type="button"
                    onClick={onOpenSettings}
                    className="text-[12px] text-[var(--color-accent-600)] hover:underline"
                  >
                    No databases configured — add URLs in Settings to search here
                  </button>
                )}
              </div>
              {selectedNoUrl.length > 0 && searchable.length > 0 && (
                <p className="mt-2 text-[11px] text-[var(--color-paper-500)]">
                  {selectedNoUrl.map((d) => d.label).join(", ")} —{" "}
                  <button
                    type="button"
                    onClick={onOpenSettings}
                    className="underline hover:text-[var(--color-ink-700)]"
                  >
                    Add URL in Settings
                  </button>
                </p>
              )}
            </section>
          );
        })()}
```

- [ ] **Step 3: Change the grid layout to accommodate the browser panel**

Find the outer grid div:
```tsx
    <div className="grid grid-cols-[1fr_320px] gap-10 max-w-6xl">
```

Replace with:
```tsx
    <div className={`grid gap-10 ${browserDb ? "grid-cols-[1fr_minmax(0,55%)]" : "grid-cols-[1fr_320px] max-w-6xl"}`}>
```

- [ ] **Step 4: Verify TypeScript compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio && npx tsc --noEmit 2>&1 | head -20
```

Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add src/views/assembling/AcquisitionForm.tsx
git commit -m "feat(browser): add Run searches row to AcquisitionForm"
```

---

## Task 5: Rust — `browser.rs` + register commands in `lib.rs`

**Files:**
- Create: `src-tauri/src/browser.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `src-tauri/src/browser.rs`**

```rust
//! Child webview browser panel — opens an external URL inside the main window
//! and intercepts reference file downloads (.ris, .bib, .txt, .csv).

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

/// Manages the single active browser panel webview.
#[derive(Default)]
pub struct BrowserState {
    pub panel: Mutex<Option<BrowserPanelData>>,
}

pub struct BrowserPanelData {
    pub webview: tauri::Webview<tauri::Wry>,
    pub temp_dir: PathBuf,
}

/// Delete all files inside the temp downloads directory (does not remove the dir itself).
pub fn clean_temp_dir(temp_dir: &PathBuf) {
    if let Ok(entries) = std::fs::read_dir(temp_dir) {
        for entry in entries.flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Create and attach a child webview to the main window.
pub fn open_panel(
    app: &tauri::AppHandle<tauri::Wry>,
    workspace: &PathBuf,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    source_database: String,
) -> Result<BrowserPanelData, String> {
    let temp_dir = workspace.join(".tmp").join("downloads");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    clean_temp_dir(&temp_dir);

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;

    let parsed_url: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;

    let app_handle = app.clone();
    let source_db = source_database.clone();
    let temp_dir_clone = temp_dir.clone();

    let webview = window
        .add_child(
            tauri::WebviewBuilder::new(
                "browser-panel",
                tauri::WebviewUrl::External(parsed_url),
            )
            .on_download(move |_webview, event| {
                match event {
                    tauri::webview::DownloadEvent::Requested { url, destination } => {
                        // Extract filename from URL path
                        let filename = url
                            .path_segments()
                            .and_then(|mut s| s.next_back())
                            .unwrap_or("download")
                            .to_string();
                        let ext = std::path::Path::new(&filename)
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        // Only intercept reference file formats
                        if matches!(ext.as_str(), "ris" | "bib" | "txt" | "csv") {
                            *destination = Some(temp_dir_clone.join(&filename));
                        }
                        true
                    }
                    tauri::webview::DownloadEvent::Finished { url: _, path, success } => {
                        if success {
                            if let Some(ref p) = path {
                                let ext = p
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("")
                                    .to_ascii_lowercase();
                                if matches!(ext.as_str(), "ris" | "bib" | "txt" | "csv") {
                                    let payload = serde_json::json!({
                                        "path": p.to_string_lossy(),
                                        "filename": p.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or(""),
                                        "source_database": source_db,
                                    });
                                    let _ = app_handle.emit("browser:download-ready", payload);
                                }
                            }
                        }
                        true
                    }
                }
            }),
            tauri::LogicalPosition::new(x, y),
            tauri::LogicalSize::new(width, height),
        )
        .map_err(|e| e.to_string())?;

    Ok(BrowserPanelData { webview, temp_dir })
}
```

- [ ] **Step 2: Add `mod browser` and `BrowserState` management to `lib.rs`**

In `src-tauri/src/lib.rs`, find the existing module declarations at the top:
```rust
mod backup;
mod clone;
mod db;
```

Add `mod browser;` at the end of the module list:
```rust
mod backup;
mod browser;
mod clone;
mod db;
mod dedup;
mod draft;
mod exports;
mod framework;
mod lmstudio;
mod lockfile;
mod parsers;
mod pdf;
mod recent;
mod suggest;
mod svg;
mod vector;
```

- [ ] **Step 3: Register `BrowserState` in `run()`**

In `src-tauri/src/lib.rs`, find:
```rust
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
```

Replace with:
```rust
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .manage(browser::BrowserState::default())
```

- [ ] **Step 4: Add the three browser Tauri commands to `lib.rs`**

Add these three command functions immediately before the `#[cfg_attr(mobile, tauri::mobile_entry_point)]` line:

```rust
#[tauri::command]
async fn open_browser_panel(
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    source_database: String,
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
    browser: State<'_, browser::BrowserState>,
) -> Result<(), String> {
    let workspace = state
        .current()
        .await
        .ok_or("no project open")?
        .workspace
        .clone();

    // Close any existing panel first
    {
        let mut guard = browser.panel.lock().map_err(|e| e.to_string())?;
        if let Some(existing) = guard.take() {
            browser::clean_temp_dir(&existing.temp_dir);
            let _ = existing.webview.close();
        }
    }

    let panel_data = browser::open_panel(
        &app,
        &workspace,
        url,
        x,
        y,
        width,
        height,
        source_database,
    )?;

    let mut guard = browser.panel.lock().map_err(|e| e.to_string())?;
    *guard = Some(panel_data);
    Ok(())
}

#[tauri::command]
fn resize_browser_panel(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    browser: State<'_, browser::BrowserState>,
) -> Result<(), String> {
    let guard = browser.panel.lock().map_err(|e| e.to_string())?;
    if let Some(ref panel) = *guard {
        panel
            .webview
            .set_position(tauri::LogicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
        panel
            .webview
            .set_size(tauri::LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn close_browser_panel(
    browser: State<'_, browser::BrowserState>,
) -> Result<(), String> {
    let mut guard = browser.panel.lock().map_err(|e| e.to_string())?;
    if let Some(panel) = guard.take() {
        browser::clean_temp_dir(&panel.temp_dir);
        let _ = panel.webview.close();
    }
    Ok(())
}
```

- [ ] **Step 5: Register the three commands in `invoke_handler!`**

Find the end of the `invoke_handler!` list:
```rust
            export_prisma_checklist,
            export_prisma_svg,
            save_draft,
            regenerate_draft,
```

Replace with:
```rust
            export_prisma_checklist,
            export_prisma_svg,
            save_draft,
            regenerate_draft,
            open_browser_panel,
            resize_browser_panel,
            close_browser_panel,
```

- [ ] **Step 6: Clean up temp downloads on project open**

In `lib.rs`, find the `open_project` function, after the `backup::snapshot` call:
```rust
    backup::snapshot(&path).ok(); // best-effort snapshot on every open
```

Add immediately after:
```rust
    // Clean up any leftover temp download files from previous sessions
    browser::clean_temp_dir(&path.join(".tmp").join("downloads"));
```

- [ ] **Step 7: Verify Rust compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -10
```

Expected: `Finished` with no errors (warnings are OK).

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/browser.rs src-tauri/src/lib.rs
git commit -m "feat(browser): Tauri child webview panel with download interception"
```

---

## Task 6: Frontend API methods + browser panel + import banners

**Files:**
- Modify: `src/lib/api.ts`
- Modify: `src/views/assembling/AcquisitionForm.tsx`

- [ ] **Step 1: Add three browser API methods to `api.ts`**

In `src/lib/api.ts`, find the closing of the `api` object (the last method before the closing `}`):
```ts
  saveDraft(content: string) {
    return invoke<string>("save_draft", { args: { content } });
  },
  regenerateDraft() {
    return invoke<string>("regenerate_draft");
  },
};
```

Replace with:
```ts
  saveDraft(content: string) {
    return invoke<string>("save_draft", { args: { content } });
  },
  regenerateDraft() {
    return invoke<string>("regenerate_draft");
  },
  openBrowserPanel(url: string, x: number, y: number, width: number, height: number, sourceDatabase: string) {
    return invoke<void>("open_browser_panel", { url, x, y, width, height, sourceDatabase: sourceDatabase });
  },
  resizeBrowserPanel(x: number, y: number, width: number, height: number) {
    return invoke<void>("resize_browser_panel", { x, y, width, height });
  },
  closeBrowserPanel() {
    return invoke<void>("close_browser_panel");
  },
};
```

- [ ] **Step 2: Add the `listen` import and `DownloadBanner` type to `AcquisitionForm.tsx`**

In `src/views/assembling/AcquisitionForm.tsx`, find:
```tsx
import { useState, useRef } from "react";
```

Replace with:
```tsx
import { useState, useRef, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
```

Then add the `DownloadBanner` interface after the imports block, before the `DATABASES` constant:
```tsx
interface DownloadBanner {
  id: number;
  path: string;
  filename: string;
  source_database: string;
  error?: string;
}
```

- [ ] **Step 3: Add browser panel state variables and ref**

In `src/views/assembling/AcquisitionForm.tsx`, find:
```tsx
  const [browserDb, setBrowserDb] = useState<string | null>(null);
  const [browserUrl, setBrowserUrl] = useState("");
```

Replace with:
```tsx
  const [browserDb, setBrowserDb] = useState<string | null>(null);
  const [browserUrl, setBrowserUrl] = useState("");
  const [downloadBanners, setDownloadBanners] = useState<DownloadBanner[]>([]);
  const panelRef = useRef<HTMLDivElement>(null);
  const bannerIdRef = useRef(0);
```

- [ ] **Step 4: Add `useEffect` for download event listener**

In `src/views/assembling/AcquisitionForm.tsx`, find the `save` function declaration:
```tsx
  const save = async () => {
```

Insert these two `useEffect` hooks immediately before it:

```tsx
  // Listen for reference files downloaded inside the browser panel
  useEffect(() => {
    const unlisten = listen<{ path: string; filename: string; source_database: string }>(
      "browser:download-ready",
      (event) => {
        bannerIdRef.current += 1;
        setDownloadBanners((prev) => [
          ...prev,
          { ...event.payload, id: bannerIdRef.current },
        ]);
      },
    );
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  // Sync the native webview with the placeholder div position
  useEffect(() => {
    if (!browserDb || !browserUrl || !panelRef.current) return;
    let observer: ResizeObserver | null = null;

    const init = () => {
      if (!panelRef.current) return;
      const rect = panelRef.current.getBoundingClientRect();
      api
        .openBrowserPanel(browserUrl, rect.left, rect.top, rect.width, rect.height, browserDb)
        .catch(() => {});

      observer = new ResizeObserver(() => {
        if (!panelRef.current) return;
        const r = panelRef.current.getBoundingClientRect();
        api.resizeBrowserPanel(r.left, r.top, r.width, r.height).catch(() => {});
      });
      observer.observe(panelRef.current);
    };

    requestAnimationFrame(init);

    return () => {
      observer?.disconnect();
      api.closeBrowserPanel().catch(() => {});
    };
  }, [browserDb, browserUrl]); // eslint-disable-line react-hooks/exhaustive-deps

```

- [ ] **Step 5: Add the browser panel placeholder and import banner JSX**

In `src/views/assembling/AcquisitionForm.tsx`, find the advisor aside block:
```tsx
      {/* Advisor side panel */}
      <aside>
        <div className="sticky top-12">
          <Card className="p-5">
```

Replace the entire `<aside>` block with:
```tsx
      {/* Right column: browser panel OR advisor card */}
      <aside>
        {browserDb ? (
          <div className="sticky top-12 flex flex-col gap-3">
            {/* Import banners for downloaded files */}
            {downloadBanners.map((banner) => (
              <motion.div
                key={banner.id}
                initial={{ opacity: 0, y: -6 }}
                animate={{ opacity: 1, y: 0 }}
                className="p-3 rounded-[var(--radius-sm)] border border-[var(--color-ok-300)] bg-[var(--color-ok-50)] flex items-center justify-between gap-3"
              >
                <div className="min-w-0">
                  <p className="text-[12px] font-medium text-[var(--color-ink-800)] truncate">
                    {banner.filename}
                  </p>
                  {banner.error ? (
                    <p className="text-[11px] text-[var(--color-warn-600)] mt-0.5">{banner.error}</p>
                  ) : (
                    <p className="text-[11px] text-[var(--color-paper-500)] mt-0.5">
                      Downloaded from {banner.source_database}
                    </p>
                  )}
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={async () => {
                      try {
                        await api.commitImport(banner.path, banner.source_database);
                        setDownloadBanners((prev) => prev.filter((b) => b.id !== banner.id));
                        onSaved();
                      } catch (e: unknown) {
                        const msg = e instanceof Error ? e.message : String(e);
                        setDownloadBanners((prev) =>
                          prev.map((b) => (b.id === banner.id ? { ...b, error: msg } : b)),
                        );
                      }
                    }}
                  >
                    Import
                  </Button>
                  <button
                    type="button"
                    onClick={() =>
                      setDownloadBanners((prev) => prev.filter((b) => b.id !== banner.id))
                    }
                    className="p-1 rounded hover:bg-[var(--color-ok-100)] transition-paper"
                    aria-label="Dismiss"
                  >
                    <X size={12} strokeWidth={2} className="text-[var(--color-paper-500)]" />
                  </button>
                </div>
              </motion.div>
            ))}

            {/* Browser panel header with close button */}
            <div className="flex items-center justify-between px-1">
              <span className="text-[11px] text-[var(--color-paper-500)] font-medium">
                {DATABASES.find((d) => d.id === browserDb)?.label ?? browserDb}
              </span>
              <button
                type="button"
                onClick={() => {
                  setBrowserDb(null);
                  setBrowserUrl("");
                }}
                className="no-drag p-1 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
                aria-label="Close browser"
              >
                <X size={14} strokeWidth={1.75} />
              </button>
            </div>

            {/* Native webview placeholder — sized to fill the column */}
            <div
              ref={panelRef}
              className="rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] bg-[var(--color-paper-100)]"
              style={{ height: "calc(100vh - 260px)", minHeight: 400 }}
            />
          </div>
        ) : (
          <div className="sticky top-12">
            <Card className="p-5">
              <div className="flex items-center gap-2 mb-3">
                <Lightbulb size={14} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                <span className="eyebrow">Advisor</span>
              </div>
              <h3 className="font-semibold text-[15px] text-[var(--color-ink-900)] mb-2 tracking-tight">
                Run the search yourself
              </h3>
              <p className="prose-editorial text-[13px] leading-[1.65] text-[var(--color-paper-700)] mb-4">
                Fangio doesn't browse library databases for you. You stay in control of your
                institutional session. When you're ready, open each selected database in your normal
                browser, paste the query into Advanced Search, set the date range, and export the
                results as RIS, BibTeX, WoS tab-delimited, or Scopus CSV.
              </p>
              <div className="hairline my-4" />
              <p className="prose-editorial text-[13px] leading-[1.65] text-[var(--color-paper-700)]">
                Then come to the <strong>Importing</strong> tab and drop the export files in. The
                dedup step will happen automatically on DOIs and you'll review the trickier
                title-fuzzy clusters yourself.
              </p>
              <div className="mt-5 px-3 py-2.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-200)]">
                <div className="flex items-center gap-1.5 text-[var(--color-paper-500)] mb-1">
                  <Globe size={11} strokeWidth={1.75} />
                  <span className="eyebrow">Why no automation</span>
                </div>
                <p className="text-[12px] text-[var(--color-paper-600)] leading-relaxed">
                  Your university policy prohibits automated retrieval from licensed databases.
                  Fangio respects that — it processes only files you've already exported yourself.
                </p>
              </div>
            </Card>
          </div>
        )}
      </aside>
```

Note: You'll also need to add the `X` icon to the lucide-react import. Find:
```tsx
import { Calendar, Database, Globe, Lightbulb, Save, Search, Sparkles } from "lucide-react";
```

Replace with:
```tsx
import { Calendar, Database, Globe, Lightbulb, Save, Search, Sparkles, X } from "lucide-react";
```

- [ ] **Step 6: Verify TypeScript compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio && npx tsc --noEmit 2>&1 | head -20
```

Expected: no output.

- [ ] **Step 7: Verify Rust compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -10
```

Expected: `Finished`.

- [ ] **Step 8: Smoke test**

```bash
npm run tauri dev
```

Verify:
1. Settings panel shows "Database access" section with 8 URL fields; Google Scholar is pre-filled
2. Acquisition tab: select Scopus → "Run searches" row appears but shows "No databases configured" (no URL set)
3. Go to Settings → paste any URL for Scopus (e.g. `https://scopus.com`) → close Settings → select Scopus in checkboxes → "Run searches" shows a Scopus button
4. Click the Scopus button → right column expands, native browser appears at that position, advisor card hidden
5. Click the X close button → advisor card restores
6. With Google Scholar URL pre-filled: select Google Scholar checkbox → "Run searches" shows Google Scholar button → clicking it opens Google Scholar in the panel
7. (If you can export a .ris from any source while the panel is open) → import banner appears in the left column above the panel header

- [ ] **Step 9: Commit**

```bash
git add src/lib/api.ts src/views/assembling/AcquisitionForm.tsx
git commit -m "feat(browser): browser panel + import banners in AcquisitionForm"
```

---

## Self-Review

**Spec coverage:**
- ✅ Database URL storage in settings — Task 1 (migration) + Task 2 (AppSettings + Settings UI)
- ✅ Google Scholar pre-filled default — Task 1 migration
- ✅ "Run searches" row with selected + URL-configured databases — Task 4
- ✅ Prompt when no URLs configured — Task 4 (the "No databases configured" button)
- ✅ "Add URL in Settings" note for selected but unconfigured databases — Task 4
- ✅ Layout change when browser open — Task 4 (grid class toggle)
- ✅ Tauri child webview at placeholder div position — Task 5 (`open_panel`) + Task 6 (useEffect + panelRef)
- ✅ ResizeObserver keeps webview aligned — Task 6 (ResizeObserver in useEffect)
- ✅ Close button restores advisor card — Task 6 (setBrowserDb(null))
- ✅ Download interception (.ris/.bib/.txt/.csv) — Task 5 (`on_download` handler)
- ✅ `browser:download-ready` event → import banner — Task 5 (Emitter) + Task 6 (listen)
- ✅ One-click import via existing pipeline — Task 6 (api.commitImport in banner)
- ✅ Dismiss banner — Task 6 (filter by id)
- ✅ Import error shown in banner — Task 6 (error state)
- ✅ Temp dir cleaned on close and on project open — Task 5 (close_browser_panel + open_project)
- ✅ Only one panel at a time — Task 5 (close existing before opening new)

**Placeholder scan:** None found.

**Type consistency:**
- `openBrowserPanel(url, x, y, width, height, sourceDatabase)` in `api.ts` → Rust receives `url: String, x: f64, y: f64, width: f64, height: f64, source_database: String` — camelCase → snake_case matches Tauri's serde rename behaviour ✓
- `DownloadBanner.id` used as React key and for filtering — consistent throughout Task 6 ✓
- `browserDb` state set to `d.id` (e.g. `"scopus"`) and used for DATABASES.find lookup — consistent ✓
- `settings[\`database.${d.id}.url\`]` pattern used in both "Run searches" row and browser open — consistent ✓
