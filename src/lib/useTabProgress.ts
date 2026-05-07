import type { TabStatus } from "../components/ProgressionTabStrip";

/**
 * Compute status for each tab.
 *   - active: tab.id === currentTab
 *   - done: dataPredicate returns true OR manual flag set
 *   - todo: otherwise
 *
 * The active tab takes precedence — even if the user has marked it complete
 * or its data predicate returns true, while they're viewing it the status
 * shows as 'active' so the underline + numbered badge remain consistent.
 */
export function useTabProgress<Id extends string>(
  currentTab: Id,
  predicates: Record<Id, () => boolean>,
  manualFlags: Record<Id, boolean>,
): Record<Id, TabStatus> {
  const result = {} as Record<Id, TabStatus>;
  for (const id in predicates) {
    if (id === currentTab) {
      result[id] = "active";
    } else {
      let done = false;
      try {
        done = predicates[id]() || !!manualFlags[id];
      } catch {
        done = !!manualFlags[id];
      }
      result[id] = done ? "done" : "todo";
    }
  }
  return result;
}

/** Read progress flags for one stage from the loaded settings dict. */
export function readManualFlags<Id extends string>(
  stage: "assembling" | "arranging" | "assessing",
  ids: Id[],
  settings: Record<string, unknown>,
): Record<Id, boolean> {
  const flags = {} as Record<Id, boolean>;
  for (const id of ids) {
    const key = `progress.${stage}.${id}.complete`;
    flags[id] = settings[key] === true;
  }
  return flags;
}
