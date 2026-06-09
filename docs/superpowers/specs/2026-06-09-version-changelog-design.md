# About Tab & In-App Changelog — Design

**Date:** 2026-06-09
**Status:** Approved

## Goal

Give Reeves an in-app "About this version" view: the running version number and
a human-readable list of what changed in each release, reachable from Settings.

## Decisions (made with the user)

| Question | Decision |
|---|---|
| Content source | Curated `CHANGELOG.md` at the repo root, rendered inside the app |
| Entry point | New **About** tab in the Settings drawer |
| Update announcements | On demand only — no popups, no badges, no last-seen-version state |
| Delivery mechanism | Vite `?raw` import: markdown baked into the JS bundle at build time |

The version number itself is already dynamic everywhere (Tauri `getVersion()`
reads `tauri.conf.json` at build time); no hardcoded versions exist or will be
added.

## Components

### 1. `CHANGELOG.md` (repo root)

Newest-first, one `## <version> — <Month Year>` section per release with
`### Added / Changed / Fixed / Removed` subsections as needed. Entries are
user-facing sentences, not commit messages. Seeded with:

- **0.2.0** — concept map tab, glossary (terms, aliases, auto-merge), cluster
  view, background tasks surviving navigation, per-feature temperature
  controls, PRISMA SVG/PNG export, markdown sanitization, removal of the
  remote companion (preserved on `with-side-companion` branch).
- **0.1.0** — brief initial-release stub.

**Release ritual:** add a section to this file before bumping the version in
`tauri.conf.json` / `package.json` / `Cargo.toml`.

### 2. Settings → About tab (`src/views/Settings.tsx`)

- Extend `SettingsTab` union and labels map: `"models" | "project" | "privacy" | "about"`.
- Tab content, top to bottom:
  1. Reeves mark (existing `carta_mark.png` asset) + app name.
  2. `Version {version}` from `useVersion()` — live, never hardcoded.
  3. The one-line app description (same copy as the bundle's shortDescription).
  4. **What's new** — `<Markdown>{changelog}</Markdown>` in a scrollable
     region, styled with the existing `markdown-body` class.
- Visual language matches the other tabs (`SectionHeader`, same type scale).

### 3. Shared `useVersion` hook (`src/lib/useVersion.ts`)

The hook currently lives privately in `StatusBar.tsx`. Move it to
`src/lib/useVersion.ts`; `StatusBar` and the About tab both import it.
Behavior unchanged: `getVersion()` once on mount, `null` until resolved or on
failure.

### 4. Changelog import

`import changelog from "../../CHANGELOG.md?raw"` in `Settings.tsx`. Vite
bundles the file contents as a string. `vite/client` types cover `?raw`
imports; if the project's tsconfig lacks them, add a one-line module
declaration in `src/vite-env.d.ts`.

## Data flow

```
CHANGELOG.md ──(vite ?raw, build time)──▶ Settings.tsx ──▶ <Markdown> (marked + DOMPurify) ──▶ About tab DOM
tauri.conf.json version ──(getVersion(), runtime)──▶ useVersion() ──▶ "Version 0.2.0"
```

## Error handling

- Missing `CHANGELOG.md` fails the **build**, never the running app.
- `getVersion()` rejection → version line simply not rendered (existing
  pattern from StatusBar).
- Markdown content is sanitized by the existing DOMPurify pass in
  `Markdown.tsx`; no new sinks.

## Out of scope

- Auto-show popups/badges on update (explicitly declined).
- Update checking / download links.
- Status-bar chip click-through (Settings tab chosen instead).

## Verification

`npx tsc --noEmit`, `npm run build`, visual check of the About tab in the dev
app (version shows, changelog renders, scrolls, headings styled).
