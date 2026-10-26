import { useState } from "react";
import { Check, CircleCheck, Loader2, RotateCcw } from "lucide-react";
import { api } from "../lib/api";
import { cn } from "../lib/utils";

interface Props {
  stage: "assembling" | "arranging" | "assessing";
  tab: string;
  /** Manually confirmed complete (persisted flag). */
  stepComplete: boolean;
  /** Auto-detected complete from the data (nothing left to do). */
  autoComplete: boolean;
  /** Short label for the step, e.g. "Title & abstract screening". */
  label: string;
  /** Shown when the step is neither confirmed nor auto-complete. */
  pendingHint: string;
  onChanged: () => void;
}

/** A confirm/override bar for a screening step: lets the user mark the step
 *  complete (or reopen it), and shows whether the data already looks done. The
 *  end-of-stage checklist reads the same persisted flag. */
export function StepConfirmBar({ stage, tab, stepComplete, autoComplete, label, pendingHint, onChanged }: Props) {
  const [busy, setBusy] = useState(false);
  const key = `progress.${stage}.${tab}.complete`;

  const setFlag = async (value: boolean) => {
    setBusy(true);
    try {
      await api.setSetting(key, value);
      onChanged();
    } finally {
      setBusy(false);
    }
  };

  if (stepComplete) {
    return (
      <div className="mb-5 flex items-center gap-3 px-4 py-2.5 rounded-[var(--radius-md)] border border-[var(--color-ok-300)] bg-[var(--color-ok-50)]">
        <CircleCheck size={16} strokeWidth={2} className="text-[var(--color-ok-600)] shrink-0" />
        <span className="flex-1 text-[12px] font-medium text-[var(--color-ok-700)]">{label} confirmed complete.</span>
        <button
          type="button"
          onClick={() => setFlag(false)}
          disabled={busy}
          className="shrink-0 inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-paper-600)] border border-[var(--color-paper-300)] bg-white hover:text-[var(--color-ink-800)] hover:border-[var(--color-paper-500)] transition-paper disabled:opacity-50"
        >
          {busy ? <Loader2 size={13} className="animate-spin" /> : <RotateCcw size={13} strokeWidth={2} />}
          Reopen
        </button>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "mb-5 flex items-center gap-3 px-4 py-2.5 rounded-[var(--radius-md)] border",
        autoComplete
          ? "border-[var(--color-ok-300)] bg-[var(--color-ok-50)]"
          : "border-[var(--color-paper-300)] bg-[var(--color-paper-50)]",
      )}
    >
      <span
        className={cn(
          "flex-1 text-[12px]",
          autoComplete ? "font-medium text-[var(--color-ok-700)]" : "text-[var(--color-paper-600)]",
        )}
      >
        {autoComplete ? `${label} looks complete — nothing left pending.` : pendingHint}
      </span>
      <button
        type="button"
        onClick={() => setFlag(true)}
        disabled={busy}
        className={cn(
          "shrink-0 inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium transition-paper disabled:opacity-50",
          autoComplete
            ? "bg-[var(--color-ok-600)] text-white hover:bg-[var(--color-ok-700)]"
            : "bg-[var(--color-accent-50)] border border-[var(--color-accent-300)] text-[var(--color-accent-700)] hover:bg-[var(--color-accent-100)]",
        )}
      >
        {busy ? <Loader2 size={13} className="animate-spin" /> : <Check size={13} strokeWidth={2} />}
        {autoComplete ? "Confirm complete" : "Mark complete anyway"}
      </button>
    </div>
  );
}
