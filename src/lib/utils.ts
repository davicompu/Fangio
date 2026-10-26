import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatRelative(iso: string): string {
  const then = new Date(iso).getTime();
  const now = Date.now();
  const diff = now - then;
  const sec = Math.round(diff / 1000);
  if (sec < 60) return "just now";
  const min = Math.round(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.round(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.round(hr / 24);
  if (day < 7) return `${day}d ago`;
  const wk = Math.round(day / 7);
  if (wk < 5) return `${wk}w ago`;
  const mo = Math.round(day / 30);
  if (mo < 12) return `${mo}mo ago`;
  return `${Math.round(day / 365)}y ago`;
}

/** Shown when the merging/consolidating model is also a coding agent. */
export const SELF_PREFERENCE_WARNING =
  "This model is also a coding agent. A model may favour its own wording when merging.";

/** Mirrors `transparency::self_preference_warning` in the Rust backend. */
export function selfPreferenceWarning(model: string, rater1: string, rater2?: string | null): boolean {
  const m = model.trim().toLowerCase();
  if (!m) return false;
  return m === rater1.trim().toLowerCase() || (!!rater2 && m === rater2.trim().toLowerCase());
}
