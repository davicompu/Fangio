# Stage Tab Progression — Design

**Status:** approved (2026-05-07)

## Problem

Each stage view (`AssemblingStage`, `ArrangingStage`, `AssessingStage`) renders its substeps as a flat tab strip. There is no visual cue that the substeps form a sequence — nothing signals which one to do first, which are complete, or which is the natural "next" step. New users land on Stage 1 and don't realise there is an order they should follow.

## Goal

Convey order and completion in the existing tab strip without restricting access. Every tab still clicks freely; the numbering, completion check, and dimmed-todo styling guide the eye and the workflow.

The signal must be tab-local — no new chrome, no new sidebar — and consistent across all three stages.

## Non-goals

- No hard gating. Every tab remains clickable from the moment the stage opens.
- No completion percentages on the tabs themselves (partial progress already lives inside each panel).
- No "next step" pulse, arrow, or animation beyond what's already shown. Add only if the visual hierarchy proves insufficient after shipping.
- No changes to the `StageHub` cards. Inter-stage gating is already handled there.

## Approach

Replace each inline tab strip with a reusable `<ProgressionTabStrip>` component. Each tab carries a small numbered badge to its left:

| State | Badge appearance | Label |
|-------|------------------|-------|
| `done` | Solid accent-500 background, white checkmark | `paper-700` text |
| `active` | Solid ink-900 background, white number | `ink-900` text + underline (existing) |
| `todo` | Accent-50 background, accent-300 border, paper-500 number | `paper-400` text |

Tabs whose completion is genuinely a judgment call (optional steps, viewers) get a small footer link inside their panel — `Mark this step complete →` — that flips the manual flag.

A `useTabProgress(stage, state)` hook returns the per-tab status map. It checks (in order): is this the current tab → `active`; does the stage's predicate return done OR is the manual flag set → `done`; else → `todo`.

### Status determination per tab

| Stage / Tab | Done when… | Manual button? |
|---|---|---|
| **Assembling** / Acquisition | `boolean_query` saved AND ≥1 database selected | No |
| Journals (optional) | — | Yes |
| Import | ≥1 `import_batch` | No |
| Library (viewer) | — | Yes |
| Dedup | All clusters reviewed (no unresolved cluster) OR manual flag | Yes |
| Checkpoint | Stage gate condition met (advanced past) | No — handled by `CheckpointGate` |
| **Arranging** / Organisation | ≥1 user-defined code in codebook | No |
| Criteria | ≥1 inclusion or exclusion criterion saved | No |
| Screening | All retrieved articles have a screening status | No |
| Coding | All included articles have ≥1 coding row | No |
| PRISMA (viewer) | — | Yes |
| **Assessing** / PDF Library | ≥80% of included articles have a PDF attached, OR manual flag | Yes |
| Ask (optional) | — | Yes |
| Draft | Draft has saved content (`draft_text` non-empty) | No |
| Export | — | Yes (terminal step) |

The 80% threshold for PDF Library reflects that perfect coverage is rarely achieved; researchers often code from a near-complete corpus.

## Components

### `<ProgressionTabStrip>` (`src/components/ProgressionTabStrip.tsx`)

```tsx
interface ProgressionTab<Id extends string> {
  id: Id;
  label: string;
  icon: React.ComponentType<{ size?: number; strokeWidth?: number; className?: string }>;
  status: "done" | "active" | "todo";
  number: number; // 1-based, left-to-right
}

interface Props<Id extends string> {
  tabs: ProgressionTab<Id>[];
  onSelect: (id: Id) => void;
  underlineLayoutId: string; // for Framer Motion shared layout
}
```

Renders the badge + icon + label + active underline. Uses the same Framer Motion `layoutId` pattern the existing tab strips use. No internal state — it's a pure render component.

### `useTabProgress(stage, ctx)` (`src/lib/useTabProgress.ts`)

```ts
type Stage = "assembling" | "arranging" | "assessing";

export function useTabProgress<Id extends string>(
  stage: Stage,
  currentTab: Id,
  predicates: Record<Id, () => boolean>,
  manualFlags: Record<Id, boolean>,
): Record<Id, "done" | "active" | "todo">;
```

The hook is intentionally minimal: it just walks the predicate map and produces statuses. Each stage view passes in its own predicates closure (which already has access to its loaded state) and the manual-flag map (loaded from settings).

### `<MarkStepComplete>` (`src/components/MarkStepComplete.tsx`)

```tsx
interface Props {
  stage: "assembling" | "arranging" | "assessing";
  tab: string;
  isComplete: boolean;
  onChanged: () => void;
}
```

Renders a small text link at the panel's footer: `Mark this step complete →` when not yet complete, `✓ Marked complete · Undo` when set. Persists via `api.saveDecision({ stage, sub_stage: tab, decision_key: "progress.<tab>.complete", value_json: "true" | "false", rationale: null })`.

## Data flow

1. Stage view mounts, fetches its existing aggregate state (`assembling_state`, project row, etc.) plus the manual-flag settings via a new helper `api.getProgressFlags(stage)` that reads from the existing settings table.
2. View constructs the predicates map closing over the loaded state.
3. View calls `useTabProgress(stage, currentTab, predicates, manualFlags)` and maps the result onto the existing `tabs` array, producing `ProgressionTab[]`.
4. Renders `<ProgressionTabStrip tabs={...} onSelect={setTab} />`.
5. Inside panels that have a manual button, `<MarkStepComplete>` writes to the decision log; the parent stage view subscribes via the existing `refreshTick` pattern and re-renders.

## Backend changes

Minimal. The existing `save_decision` and `all_settings` commands already cover what we need — manual flags are just a set of `progress.<tab>.complete = "true"` rows in the settings table, with stage as the `sub_stage` field.

Add one tiny helper command for ergonomics:

```rust
#[tauri::command]
async fn get_progress_flags(stage: String, state: State<'_, AppState>) -> Result<HashMap<String, bool>, String>
```

It reads all decision rows for the given stage where the `decision_key` starts with `progress.` and ends with `.complete`, returning a flat `tab_id → bool` map. (Pure convenience; the same data could be derived client-side from `all_settings`.)

## Persistence semantics

- Manual flags persist in the project's SQLite database alongside other decisions. Closing and reopening the project preserves them.
- Data-driven predicates re-evaluate on every render. If a user deletes their last code, the Organisation tab loses its checkmark.
- Manual flags are independent of data predicates: a tab marked complete stays checked even if the underlying data later changes (the flag wins). This matches user intent — they explicitly said it was done.

## Visual / styling notes

- Badge size: `w-4.5 h-4.5` (18px). Centered numeric content uses `font-mono text-[10px] font-bold`.
- Done check: 10×10 SVG, white stroke, weight 1.6, rounded line caps.
- Spacing: badge sits 8px left of the icon (existing icon-to-label gap is unchanged).
- Tab `padding`/`height` unchanged so the existing tab-strip layout doesn't shift.
- Active underline (Framer Motion `layoutId`) unchanged in behaviour and appearance.

## Error handling

- If `get_progress_flags` fails, fall back to all-false manual flags. Log to console; don't disrupt the UI.
- If a predicate throws (e.g., the project row is malformed), default that tab to `todo`. The tab is still clickable.
- Saving a manual flag that fails network-side leaves the visible state unchanged and surfaces a toast (existing pattern).

## Testing

Manual smoke test:

1. Open a brand-new project that has just left onboarding. Enter Stage 1.
   - Confirm: tabs are numbered 1–6, all `todo`, active tab is Acquisition (1).
2. Save acquisition decisions (databases + boolean query).
   - Confirm: Acquisition badge shows a check.
3. Click Journals (optional), then click `Mark this step complete`.
   - Confirm: Journals badge shows a check.
4. Click an empty Library; confirm no check, click `Mark this step complete`, confirm check appears.
5. Reload the project. Confirm the Acquisition + Journals + Library checks persist.
6. Repeat for Arranging and Assessing. Verify the data-driven predicates fire as expected (e.g., add a code → Organisation flips to done).
7. Resize the window narrow. Verify the badges don't break the tab strip layout.

No automated tests are added — the wizard already has none and adding a Vitest setup is out of scope.

## Future work

- "Next step" emphasis (subtle pulsing border on the first todo tab) if onboarding research shows the cue is too subtle.
- Per-tab progress hint inside the panel header ("3/47 articles screened" inline with the badge).
- Decision-log entries for manual completes could feed a per-stage timeline view.
