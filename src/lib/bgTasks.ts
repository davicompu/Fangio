// Background-task store for long-running "Ask the Literature" operations.
//
// The problem this solves: when the user clicks Back to leave the RAG view
// while a cluster build / glossary extract / RAG index build is in flight,
// the Rust async command keeps running but the React event listener gets
// torn down and the result is lost.
//
// Fix: a tiny module-level store. Listeners are registered ONCE (from
// App.tsx via useBgTaskBootstrap) and survive every component
// mount/unmount. Operations are kicked off through the helpers below; UI
// components subscribe via useBgTask() and just read whatever the latest
// state is.
//
// Cleared on full app restart (no on-disk persistence — these tasks finish
// in seconds-to-minutes and the cost of re-running is acceptable).

import { useSyncExternalStore } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  api,
  type BuildRagProgress,
  type BuildRagResult,
  type ClusterResult,
  type ClusterProgress,
  type GlossaryProgress,
} from "./api";

export type TaskKind = "cluster" | "glossary" | "ragBuild";

interface TaskBase {
  /** True while the underlying Rust command hasn't returned yet. */
  running: boolean;
  /** Last error message from a failed run, or null. */
  error: string | null;
  /** Wall-clock start of the most recent run (ms since epoch), or null. */
  startedAt: number | null;
  /** Wall-clock finish of the most recent run (ms since epoch), or null. */
  finishedAt: number | null;
}

export interface ClusterTaskState extends TaskBase {
  progress: ClusterProgress | null;
  result: ClusterResult | null;
}

export interface GlossaryTaskState extends TaskBase {
  progress: GlossaryProgress | null;
  /** glossary_term_count returned from the Rust extract_glossary command. */
  result: number | null;
  /** "rebuild" wipes existing rows; "update" only adds missing articles. */
  mode: "rebuild" | "update" | null;
}

export interface RagBuildTaskState extends TaskBase {
  progress: BuildRagProgress | null;
  /** Article + chunk counts from the most recent run. */
  result: BuildRagResult | null;
}

interface State {
  cluster: ClusterTaskState;
  glossary: GlossaryTaskState;
  ragBuild: RagBuildTaskState;
}

const initialBase: TaskBase = {
  running: false,
  error: null,
  startedAt: null,
  finishedAt: null,
};

let state: State = {
  cluster: { ...initialBase, progress: null, result: null },
  glossary: { ...initialBase, progress: null, result: null, mode: null },
  ragBuild: { ...initialBase, progress: null, result: null },
};

const listeners = new Set<() => void>();

function setState(next: State) {
  state = next;
  for (const fn of listeners) fn();
}

function update<K extends TaskKind>(kind: K, patch: Partial<State[K]>): void {
  setState({ ...state, [kind]: { ...state[kind], ...patch } });
}

// ── React hooks ────────────────────────────────────────────────────────────

const subscribe = (fn: () => void) => {
  listeners.add(fn);
  return () => listeners.delete(fn);
};

export function useBgTasks(): State {
  return useSyncExternalStore(subscribe, () => state, () => state);
}

export function useClusterTask(): ClusterTaskState {
  return useSyncExternalStore(subscribe, () => state.cluster, () => state.cluster);
}

export function useGlossaryTask(): GlossaryTaskState {
  return useSyncExternalStore(subscribe, () => state.glossary, () => state.glossary);
}

export function useRagBuildTask(): RagBuildTaskState {
  return useSyncExternalStore(subscribe, () => state.ragBuild, () => state.ragBuild);
}

// ── One-time listener bootstrap ────────────────────────────────────────────
//
// These listeners stay registered for the lifetime of the app. Tauri delivers
// every progress event regardless of which React view is mounted, so progress
// is captured even when the user navigates away from the RAG view.

let bootstrapped = false;

export async function bootstrapBgTaskListeners(): Promise<UnlistenFn[]> {
  if (bootstrapped) return [];
  bootstrapped = true;
  const unlisteners: UnlistenFn[] = [];
  unlisteners.push(
    await listen<ClusterProgress>("clusters:progress", (e) => update("cluster", { progress: e.payload })),
  );
  unlisteners.push(
    await listen<GlossaryProgress>("glossary:progress", (e) => update("glossary", { progress: e.payload })),
  );
  unlisteners.push(
    await listen<BuildRagProgress>("rag:progress", (e) => update("ragBuild", { progress: e.payload })),
  );
  return unlisteners;
}

// ── Operation kickoffs ─────────────────────────────────────────────────────
//
// These are fire-and-forget — they update the store as the Rust command
// resolves. Callers (UI components) call them and then watch the store via
// the hooks above. They never throw; errors land in `state.<kind>.error`.

/** k: number of clusters, or null for automatic. */
export function startClusterBuild(k: number | null): void {
  if (state.cluster.running) return;
  update("cluster", {
    running: true,
    error: null,
    progress: null,
    result: null,
    startedAt: Date.now(),
    finishedAt: null,
  });
  api.clusterCorpus(k)
    .then((clusters) => update("cluster", { running: false, result: clusters, finishedAt: Date.now() }))
    .catch((e) => update("cluster", { running: false, error: e instanceof Error ? e.message : String(e), finishedAt: Date.now() }));
}

export function startGlossaryExtract(mode: "rebuild" | "update"): void {
  if (state.glossary.running) return;
  update("glossary", {
    running: true,
    error: null,
    progress: null,
    result: null,
    mode,
    startedAt: Date.now(),
    finishedAt: null,
  });
  api.extractGlossary(mode)
    .then((count) => update("glossary", { running: false, result: count, finishedAt: Date.now() }))
    .catch((e) => update("glossary", { running: false, error: e instanceof Error ? e.message : String(e), finishedAt: Date.now() }));
}

export function startRagBuild(): void {
  if (state.ragBuild.running) return;
  update("ragBuild", {
    running: true,
    error: null,
    progress: null,
    result: null,
    startedAt: Date.now(),
    finishedAt: null,
  });
  api.buildRagIndex()
    .then((res) => update("ragBuild", { running: false, result: res, finishedAt: Date.now() }))
    .catch((e) => update("ragBuild", { running: false, error: e instanceof Error ? e.message : String(e), finishedAt: Date.now() }));
}

/** Reset the in-memory state for one task (e.g. user dismissed the result). */
export function clearTask(kind: TaskKind): void {
  if (kind === "cluster") {
    update("cluster", { progress: null, result: null, error: null, startedAt: null, finishedAt: null });
  } else if (kind === "glossary") {
    update("glossary", { progress: null, result: null, error: null, startedAt: null, finishedAt: null, mode: null });
  } else if (kind === "ragBuild") {
    update("ragBuild", { progress: null, result: null, error: null, startedAt: null, finishedAt: null });
  }
}
