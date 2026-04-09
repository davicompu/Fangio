# Editable Research Questions on Stage Hub — Design Spec

**Date:** 2026-04-09  
**Component affected:** `src/views/StageHub.tsx`  
**Feature:** Allow the user to edit rq_what, rq_how, rq_where directly from the main hub page

---

## Overview

The research questions card on the Stage Hub is currently read-only. Add an "Edit" button to the card header so the user can update all three research questions in place without going back through onboarding.

---

## Architecture

No backend changes required. The `saveDecision` API with keys `project.rq_what`, `project.rq_how`, `project.rq_where` already writes to the project row (verified in `db.rs:247`).

All changes are confined to `src/views/StageHub.tsx` and a one-line addition in `src/App.tsx`.

---

## Changes

### `src/App.tsx`

Add `onProjectChanged={refreshProject}` to the `<StageHub>` JSX (line 69). `refreshProject` is already returned from `useAppState` and re-fetches the current project row then calls `setProject`.

```tsx
<StageHub
  project={project}
  onOpenSettings={() => setSettingsOpen(true)}
  onProjectChanged={refreshProject}
  onEnterStage={(s) => { ... }}
/>
```

### `src/views/StageHub.tsx`

**New prop on `StageHubProps`:**
```ts
onProjectChanged: () => void;
```

**New state inside `StageHub`:**
```ts
const [editing, setEditing] = useState(false);
const [draftWhat, setDraftWhat] = useState("");
const [draftHow, setDraftHow] = useState("");
const [draftWhere, setDraftWhere] = useState("");
const [saving, setSaving] = useState(false);
const [savedAt, setSavedAt] = useState<number | null>(null);
```

**Edit button:** A small pencil-icon button (`Pencil` from lucide-react, `ghost` style) placed right-aligned in the card header, next to the "Research questions" eyebrow. Only visible when `!editing`.

**On Edit click:**
- Populate draft state from `project.rq_what/how/where` (falling back to `""`)
- Set `editing = true`

**Edit mode card body:** The three `RqLine` displays are replaced by three `TextArea` fields (imported from `../../components/ui/TextField`) labelled "What", "How", "Where" with their existing onboarding placeholder text.

**Save handler:**
- Sets `saving = true`
- Calls `api.saveDecision` for each field that has a non-empty trimmed value, using keys `project.rq_what`, `project.rq_how`, `project.rq_where` and `stage: "onboarding"`, `sub_stage: null`
- Calls `onProjectChanged()` to re-fetch the updated project row
- Sets `savedAt = Date.now()`, `editing = false`, `saving = false`

**Cancel handler:**
- Sets `editing = false`, clears draft state — no API calls

**"Saved" confirmation:** A `motion.span` with `"Saved to decision log"` fades in next to the Save/Cancel buttons after save, identical to the pattern in `AcquisitionForm.tsx`.

**Imports to add:** `useState` (already imported), `Pencil` from lucide-react, `TextArea` from `../../components/ui/TextField`, `Button` from `../../components/ui/Button`, `motion` from framer-motion (already imported), `api` from `../lib/api`.

---

## Data Flow

```
User clicks Edit
  → draft state populated from project prop
  → editing = true → card shows TextArea fields

User edits fields and clicks Save
  → api.saveDecision("project.rq_what", ...)
  → api.saveDecision("project.rq_how", ...)
  → api.saveDecision("project.rq_where", ...)
  → onProjectChanged() → App.refreshProject() → setProject(updated)
  → editing = false, savedAt = Date.now()
  → card returns to read-only with updated values
  → "Saved to decision log" flash fades in then out

User clicks Cancel
  → editing = false, drafts discarded
```

---

## Out of Scope

- Editing the domain paragraph (the italic text below the hairline) — could be a future follow-up
- Validation (the onboarding wizard validates length; this edit form does not — intentional, the user already completed onboarding)
- Editing from within a stage view (Assembling, Arranging, Assessing) — hub only
