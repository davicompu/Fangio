# Editable Research Questions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an Edit button to the research questions card on the Stage Hub so users can update rq_what, rq_how, and rq_where without going back through onboarding.

**Architecture:** All changes are in two files — a one-line prop addition in `App.tsx` and edit/save logic added to `StageHub.tsx`. No backend changes needed; `api.saveDecision` with keys `project.rq_what/how/where` already writes to the project row.

**Tech Stack:** React/TypeScript, Tauri `invoke` (via existing `api`), Framer Motion (animations), Lucide icons, Tailwind CSS

---

## File Map

| Action | Path | Purpose |
|--------|------|---------|
| Modify | `src/App.tsx` | Pass `onProjectChanged={refreshProject}` to `StageHub` |
| Modify | `src/views/StageHub.tsx` | Add `onProjectChanged` prop, edit state, handlers, edit-mode card UI |

---

## Task 1: Add `onProjectChanged` prop to `StageHub` and wire it in `App.tsx`

**Files:**
- Modify: `src/App.tsx` (line 69–78)
- Modify: `src/views/StageHub.tsx` (lines 17–20, 51)

- [ ] **Step 1: Add `onProjectChanged` to `StageHubProps` in `StageHub.tsx`**

Find the interface (lines 17–21):
```tsx
interface StageHubProps {
  project: ProjectRow;
  onOpenSettings: () => void;
  onEnterStage: (stage: "assembling" | "arranging" | "assessing") => void;
}
```

Replace with:
```tsx
interface StageHubProps {
  project: ProjectRow;
  onOpenSettings: () => void;
  onProjectChanged: () => void;
  onEnterStage: (stage: "assembling" | "arranging" | "assessing") => void;
}
```

- [ ] **Step 2: Destructure `onProjectChanged` in the `StageHub` function signature**

Find (line 51):
```tsx
export function StageHub({ project, onOpenSettings, onEnterStage }: StageHubProps) {
```

Replace with:
```tsx
export function StageHub({ project, onOpenSettings, onProjectChanged, onEnterStage }: StageHubProps) {
```

- [ ] **Step 3: Pass `onProjectChanged={refreshProject}` in `App.tsx`**

Find the `useAppState` destructure (lines 16–24) — `refreshProject` is already returned but not currently destructured at the call site. Add it:
```tsx
  const {
    project,
    setProject,
    settings,
    updateSetting,
    lm,
    checkLmStudio,
    refreshProject,
    loading,
  } = useAppState();
```

Then find the `<StageHub>` JSX (lines 69–78):
```tsx
            <StageHub
              project={project}
              onOpenSettings={() => setSettingsOpen(true)}
              onEnterStage={(s) => {
                if (s === "assembling") setRoute("assembling");
                if (s === "arranging") setRoute("arranging");
                if (s === "assessing") setRoute("assessing");
              }}
            />
```

Replace with:
```tsx
            <StageHub
              project={project}
              onOpenSettings={() => setSettingsOpen(true)}
              onProjectChanged={refreshProject}
              onEnterStage={(s) => {
                if (s === "assembling") setRoute("assembling");
                if (s === "arranging") setRoute("arranging");
                if (s === "assessing") setRoute("assessing");
              }}
            />
```

- [ ] **Step 4: Verify TypeScript compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio && npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx src/views/StageHub.tsx
git commit -m "feat(hub): add onProjectChanged prop to StageHub"
```

---

## Task 2: Add edit state, handlers, and edit-mode UI to `StageHub.tsx`

**Files:**
- Modify: `src/views/StageHub.tsx`

- [ ] **Step 1: Add new imports to `StageHub.tsx`**

Find the existing import block at the top. Make these additions:

Add `useState` to the React import (it is not currently imported — the file uses no hooks):
```tsx
import { useState } from "react";
```

Add `Pencil` and `Save` to the lucide-react import. Find:
```tsx
import {
  ArrowUpRight,
  Boxes,
  CheckCircle2,
  Compass,
  Lock,
  PenLine,
  Quote,
  Search,
  Settings as SettingsIcon,
} from "lucide-react";
```

Replace with:
```tsx
import {
  ArrowUpRight,
  Boxes,
  CheckCircle2,
  Compass,
  Lock,
  PenLine,
  Pencil,
  Quote,
  Save,
  Search,
  Settings as SettingsIcon,
} from "lucide-react";
```

Add imports for `api`, `Button`, `TextArea`, and `motion` (motion is already imported via framer-motion — confirm it's there; if not add it). Add after the existing Card import:
```tsx
import { api } from "../lib/api";
import { Button } from "../components/ui/Button";
import { TextArea } from "../components/ui/TextField";
```

- [ ] **Step 2: Add edit state variables inside the `StageHub` function**

Find the line `const currentStage = project.current_stage;` (line 52). Add the new state immediately after it:

```tsx
  const currentStage = project.current_stage;

  const [editing, setEditing] = useState(false);
  const [draftWhat, setDraftWhat] = useState("");
  const [draftHow, setDraftHow] = useState("");
  const [draftWhere, setDraftWhere] = useState("");
  const [saving, setSaving] = useState(false);
  const [savedAt, setSavedAt] = useState<number | null>(null);
```

- [ ] **Step 3: Add `handleEdit`, `handleSave`, and `handleCancel` functions**

Add these three functions after the state declarations (before the `return`):

```tsx
  const handleEdit = () => {
    setDraftWhat(project.rq_what ?? "");
    setDraftHow(project.rq_how ?? "");
    setDraftWhere(project.rq_where ?? "");
    setSavedAt(null);
    setEditing(true);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      if (draftWhat.trim()) {
        await api.saveDecision({
          stage: "onboarding",
          sub_stage: null,
          decision_key: "project.rq_what",
          value_json: JSON.stringify(draftWhat.trim()),
          rationale: null,
        });
      }
      if (draftHow.trim()) {
        await api.saveDecision({
          stage: "onboarding",
          sub_stage: null,
          decision_key: "project.rq_how",
          value_json: JSON.stringify(draftHow.trim()),
          rationale: null,
        });
      }
      if (draftWhere.trim()) {
        await api.saveDecision({
          stage: "onboarding",
          sub_stage: null,
          decision_key: "project.rq_where",
          value_json: JSON.stringify(draftWhere.trim()),
          rationale: null,
        });
      }
      await onProjectChanged();
      setSavedAt(Date.now());
      setEditing(false);
    } finally {
      setSaving(false);
    }
  };

  const handleCancel = () => {
    setEditing(false);
    setDraftWhat("");
    setDraftHow("");
    setDraftWhere("");
  };
```

- [ ] **Step 4: Replace the research questions card JSX**

Find the entire research questions card block (lines 81–108):
```tsx
      {/* Research Question summary card — anchors the whole project */}
      <div className="px-12 mb-10">
        <Card className="p-7">
          <div className="flex items-center gap-2 mb-4 text-[var(--color-paper-500)]">
            <Compass size={13} strokeWidth={1.75} />
            <span className="eyebrow">Research questions</span>
          </div>
          <div className="space-y-4">
            <RqLine label="What" body={project.rq_what || ""} />
            <RqLine label="How" body={project.rq_how || ""} />
            <RqLine label="Where" body={project.rq_where || ""} />
          </div>
          {project.domain && (
            <>
              <div className="my-5 hairline" />
              <div className="flex items-start gap-3">
                <Quote
                  size={13}
                  strokeWidth={1.75}
                  className="text-[var(--color-paper-500)] mt-1 shrink-0"
                />
                <p className="prose-editorial text-[14px] text-[var(--color-paper-700)] italic leading-relaxed">
                  {project.domain}
                </p>
              </div>
            </>
          )}
        </Card>
      </div>
```

Replace with:
```tsx
      {/* Research Question summary card — anchors the whole project */}
      <div className="px-12 mb-10">
        <Card className="p-7">
          {/* Card header */}
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2 text-[var(--color-paper-500)]">
              <Compass size={13} strokeWidth={1.75} />
              <span className="eyebrow">Research questions</span>
            </div>
            {!editing && (
              <button
                type="button"
                onClick={handleEdit}
                className="no-drag flex items-center gap-1.5 px-2.5 py-1.5 rounded-[var(--radius-xs)] text-[11px] font-medium text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] hover:bg-[var(--color-paper-100)] transition-paper"
              >
                <Pencil size={11} strokeWidth={1.75} />
                Edit
              </button>
            )}
          </div>

          {/* Read-only display */}
          {!editing && (
            <>
              <div className="space-y-4">
                <RqLine label="What" body={project.rq_what || ""} />
                <RqLine label="How" body={project.rq_how || ""} />
                <RqLine label="Where" body={project.rq_where || ""} />
              </div>
              {project.domain && (
                <>
                  <div className="my-5 hairline" />
                  <div className="flex items-start gap-3">
                    <Quote
                      size={13}
                      strokeWidth={1.75}
                      className="text-[var(--color-paper-500)] mt-1 shrink-0"
                    />
                    <p className="prose-editorial text-[14px] text-[var(--color-paper-700)] italic leading-relaxed">
                      {project.domain}
                    </p>
                  </div>
                </>
              )}
            </>
          )}

          {/* Edit mode */}
          {editing && (
            <div className="space-y-4">
              <TextArea
                label="What do we know about this domain?"
                value={draftWhat}
                onChange={(e) => setDraftWhat(e.target.value)}
                rows={3}
                placeholder="e.g., What is currently known about the antecedents, behaviours, and outcomes of consumer engagement in social commerce platforms?"
              />
              <TextArea
                label="How do we know what we know?"
                value={draftHow}
                onChange={(e) => setDraftHow(e.target.value)}
                rows={3}
                placeholder="e.g., What theories, contexts, and methods have been used to study consumer behaviour in social commerce, and how has methodological rigour evolved?"
              />
              <TextArea
                label="Where should this field be heading?"
                value={draftWhere}
                onChange={(e) => setDraftWhere(e.target.value)}
                rows={3}
                placeholder="e.g., What gaps, contradictions, and unexplored contexts should future research on consumer behaviour in social commerce address?"
              />
              <div className="flex items-center gap-3 pt-2">
                <Button onClick={handleSave} disabled={saving} variant="primary" size="sm">
                  <Save size={13} strokeWidth={2} />
                  {saving ? "Saving…" : "Save"}
                </Button>
                <Button onClick={handleCancel} disabled={saving} variant="ghost" size="sm">
                  Cancel
                </Button>
                {savedAt && (
                  <motion.span
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    className="text-[12px] text-[var(--color-ok-500)]"
                  >
                    Saved to decision log
                  </motion.span>
                )}
              </div>
            </div>
          )}
        </Card>
      </div>
```

- [ ] **Step 5: Verify TypeScript compiles**

```bash
cd /Users/david/Documents/GitHub/Fangio && npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors.

- [ ] **Step 6: Smoke test manually**

```bash
npm run tauri dev
```

Verify:
1. Hub shows the RQ card with an "Edit" button in the top-right of the card
2. Click Edit — three text areas appear pre-filled with current values; domain paragraph is hidden
3. Edit one field and click Save — card returns to read-only with the updated text; "Saved to decision log" flashes
4. Click Edit again, make no changes, click Cancel — card returns to read-only unchanged
5. The "Suggest" / "Advise" button in Stage 1 · Assembling uses the updated RQ values (they're now in the project row)

- [ ] **Step 7: Commit**

```bash
git add src/views/StageHub.tsx
git commit -m "feat(hub): editable research questions card with save/cancel"
```

---

## Self-Review

**Spec coverage:**
- ✅ Edit button on card header — Task 2 Step 4
- ✅ All three fields editable at once — Task 2 Step 4 (three TextArea fields)
- ✅ Pre-populated from `project` on Edit click — Task 2 Step 3 `handleEdit`
- ✅ Save calls `api.saveDecision` for each field — Task 2 Step 3 `handleSave`
- ✅ Calls `onProjectChanged()` after save — Task 2 Step 3 `handleSave`
- ✅ Returns to read-only after save — `setEditing(false)` in `handleSave`
- ✅ "Saved to decision log" confirmation — Task 2 Step 4 `savedAt` motion.span
- ✅ Cancel discards draft, no API calls — Task 2 Step 3 `handleCancel`
- ✅ `onProjectChanged` prop added to interface — Task 1 Step 1
- ✅ `refreshProject` passed from `App.tsx` — Task 1 Step 3

**Placeholder scan:** None found.

**Type consistency:**
- `onProjectChanged: () => void` defined in Task 1 Step 1, destructured in Task 1 Step 2, called in Task 2 Step 3 — consistent
- `handleEdit`, `handleSave`, `handleCancel` defined in Task 2 Step 3, referenced in Task 2 Step 4 — consistent
- `draftWhat/How/Where` state defined in Task 2 Step 2, used in Step 3 and Step 4 — consistent
