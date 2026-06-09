# About Tab & In-App Changelog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Settings → About tab showing the live app version and a "What's new" changelog rendered from a repo-root `CHANGELOG.md` baked in at build time.

**Architecture:** A curated `CHANGELOG.md` is imported into the frontend with Vite's `?raw` suffix and rendered through the existing sanitized `Markdown` component. The private `useVersion()` hook moves out of `StatusBar.tsx` into `src/lib/useVersion.ts` so the About tab and status bar share it. Settings gains a fourth tab. No backend changes.

**Tech Stack:** React + TypeScript, Vite `?raw` imports, Tauri `getVersion()`, existing `Markdown` (marked + DOMPurify) component, lucide icons.

**Spec:** `docs/superpowers/specs/2026-06-09-version-changelog-design.md`

**Verification model:** This repo has no JS test runner (no vitest/jest). Per the spec, the gates are `npx tsc --noEmit` and `npm run build` after each task, plus a visual check at the end. Do not add a test framework.

---

## Files Changed

| File | Change |
|---|---|
| `CHANGELOG.md` | Create: seeded with 0.2.0 + 0.1.0 entries |
| `src/lib/useVersion.ts` | Create: shared version hook |
| `src/components/StatusBar.tsx` | Modify: import shared hook, delete local copy |
| `src/views/Settings.tsx` | Modify: add `about` tab (union, labels, strip, content) |
| `src/vite-env.d.ts` | Modify only if `tsc` rejects the `?raw` import (exact fallback in Task 3) |

---

## Task 1: Create CHANGELOG.md

**Files:**
- Create: `CHANGELOG.md`

- [ ] **Step 1: Create the file with this exact content**

```markdown
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
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: seed CHANGELOG.md with 0.1.0 and 0.2.0 release notes"
```

---

## Task 2: Shared useVersion hook

**Files:**
- Create: `src/lib/useVersion.ts`
- Modify: `src/components/StatusBar.tsx` (lines 1–5 imports, lines 44–51 local hook)

- [ ] **Step 1: Create `src/lib/useVersion.ts` with this exact content**

```typescript
import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

/**
 * The running app version from tauri.conf.json (e.g. "0.2.0"), or null until
 * resolved / if the IPC call fails. Shared by StatusBar and Settings → About.
 */
export function useVersion(): string | null {
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);
  return version;
}
```

- [ ] **Step 2: Update `src/components/StatusBar.tsx`**

Remove the `getVersion` import (line 4) and add the hook import. The imports block currently reads:

```typescript
import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { Brain, Database, Gauge, Radio, WifiOff } from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import { cn } from "../lib/utils";
```

Change it to:

```typescript
import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { Brain, Database, Gauge, Radio, WifiOff } from "lucide-react";
import { cn } from "../lib/utils";
import { useVersion } from "../lib/useVersion";
```

(`useEffect`/`useState` stay — `useClock` and the `pinned` state still use them.)

Then delete the local hook (lines 44–51):

```typescript
function useVersion() {
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);
  return version;
}
```

The call site `const version = useVersion();` inside the component is unchanged.

- [ ] **Step 3: Verify**

Run: `npx tsc --noEmit`
Expected: exit 0, no output.

- [ ] **Step 4: Commit**

```bash
git add src/lib/useVersion.ts src/components/StatusBar.tsx
git commit -m "refactor: extract useVersion hook to src/lib for reuse"
```

---

## Task 3: Settings → About tab

**Files:**
- Modify: `src/views/Settings.tsx` (imports ~lines 6–29, tab union line 39, tab strip lines 175–181, new content block after the privacy section ~line 431)
- Modify (only if needed): `src/vite-env.d.ts`

- [ ] **Step 1: Add imports to `src/views/Settings.tsx`**

Add `Info` to the existing lucide-react import list (it currently contains `Cpu, Database, Filter, GripVertical, HardDrive, Loader2, Pencil, Plus, Radio, Thermometer, Users, X` — keep alphabetical order, so `Info` goes after `HardDrive`):

```typescript
  HardDrive,
  Info,
  Loader2,
```

Below the existing imports add:

```typescript
import { Markdown } from "../components/Markdown";
import { useVersion } from "../lib/useVersion";
import cartaMark from "../assets/carta_mark.png";
import changelog from "../../CHANGELOG.md?raw";
```

- [ ] **Step 2: Extend the tab union (line 39)**

```typescript
  type SettingsTab = "models" | "project" | "privacy" | "about";
```

- [ ] **Step 3: Read the version inside the component**

Immediately after `const [tab, setTab] = useState<SettingsTab>("models");` add:

```typescript
  const version = useVersion();
```

- [ ] **Step 4: Add the tab to the strip (line ~175)**

Current:

```typescript
              {(["models", "project", "privacy"] as const).map((t) => {
                const labels: Record<SettingsTab, string> = {
                  models: "Models",
                  project: "Project",
                  privacy: "Privacy",
                };
```

Change to:

```typescript
              {(["models", "project", "privacy", "about"] as const).map((t) => {
                const labels: Record<SettingsTab, string> = {
                  models: "Models",
                  project: "Project",
                  privacy: "Privacy",
                  about: "About",
                };
```

- [ ] **Step 5: Add the tab content**

Insert this block directly after the privacy section's closing `)}` (the section guarded by `{tab === "privacy" && (`, which ends around line 431), before the `{/* ── Project & backups ─── */}` comment:

```tsx
              {/* ── About ────────────────────────────────────────────────── */}
              {tab === "about" && (
              <section>
                <SectionHeader
                  icon={<Info size={14} strokeWidth={1.75} />}
                  title="About Reeves"
                  description="Systematic literature reviews, conducted with rigor and on your own machine."
                />

                <div className="mt-5 space-y-5">
                  <div className="flex items-center gap-3">
                    <img
                      src={cartaMark}
                      alt="Reeves"
                      className="w-10 h-10 rounded-[var(--radius-xs)]"
                    />
                    <div>
                      <p className="text-[15px] font-medium text-[var(--color-ink-800)]">Reeves</p>
                      {version && (
                        <p className="font-mono text-[12px] text-[var(--color-paper-500)]">
                          Version {version}
                        </p>
                      )}
                    </div>
                  </div>

                  <div>
                    <p className="eyebrow text-[var(--color-paper-500)] mb-2">What&rsquo;s new</p>
                    <div className="max-h-[340px] overflow-y-auto rounded-[var(--radius-xs)] border border-[var(--color-paper-200)] bg-white p-4">
                      <Markdown className="markdown-sm">{changelog}</Markdown>
                    </div>
                  </div>
                </div>
              </section>
              )}
```

- [ ] **Step 6: Typecheck**

Run: `npx tsc --noEmit`
Expected: exit 0.

**If and only if** tsc fails with `Cannot find module '../../CHANGELOG.md?raw'`, append this to `src/vite-env.d.ts` and re-run:

```typescript
declare module "*.md?raw" {
  const content: string;
  export default content;
}
```

- [ ] **Step 7: Production build**

Run: `npm run build`
Expected: `✓ built` with exit 0 (the chunk-size warning is pre-existing and fine).

- [ ] **Step 8: Commit**

```bash
git add src/views/Settings.tsx src/vite-env.d.ts
git commit -m "feat(about): Settings About tab with live version + what's-new changelog"
```

---

## Task 4: Visual verification

**Files:** none (verification only)

- [ ] **Step 1: Launch the dev app**

Run: `npm run tauri dev` (leave running; it opens the Reeves window).

- [ ] **Step 2: Check the About tab**

Open a project → gear icon (Settings) → **About** tab. Confirm:
- Reeves mark renders, "Version 0.2.0" shows under the name.
- "What's new" shows the 0.2.0 section with styled headings and bullets.
- The changelog region scrolls; other tabs unaffected.

- [ ] **Step 3: Stop the dev app**

Ctrl-C the dev process. No commit (no changes).
