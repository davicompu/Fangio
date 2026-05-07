import { useState } from "react";
import { Check, Loader2, RotateCcw } from "lucide-react";
import { api } from "../lib/api";
import { cn } from "../lib/utils";

interface Props {
  stage: "assembling" | "arranging" | "assessing";
  tab: string;
  isComplete: boolean;
  onChanged: () => void;
}

export function MarkStepComplete({ stage, tab, isComplete, onChanged }: Props) {
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

  if (isComplete) {
    return (
      <div className="flex items-center gap-2">
        <div
          className={cn(
            "flex-1 inline-flex items-center justify-center gap-2 px-4 py-2.5",
            "rounded-[var(--radius-sm)] text-[12px] font-medium",
            "bg-[var(--color-ok-50)] border border-[var(--color-ok-500)] text-[var(--color-ok-700)]",
          )}
        >
          <Check size={14} strokeWidth={2.5} />
          Step complete
        </div>
        <button
          type="button"
          onClick={() => setFlag(false)}
          disabled={busy}
          title="Undo — reopen this step"
          aria-label="Undo step complete"
          className={cn(
            "shrink-0 inline-flex items-center justify-center w-9 h-9 rounded-[var(--radius-sm)]",
            "border border-[var(--color-paper-300)] bg-white text-[var(--color-paper-600)]",
            "hover:border-[var(--color-paper-500)] hover:text-[var(--color-ink-800)] transition-paper",
            "disabled:opacity-50 disabled:cursor-wait",
          )}
        >
          {busy ? <Loader2 size={13} className="animate-spin" /> : <RotateCcw size={13} strokeWidth={2} />}
        </button>
      </div>
    );
  }

  return (
    <button
      type="button"
      onClick={() => setFlag(true)}
      disabled={busy}
      className={cn(
        "w-full inline-flex items-center justify-center gap-2 px-4 py-2.5",
        "rounded-[var(--radius-sm)] text-[12px] font-medium transition-paper",
        "bg-[var(--color-accent-50)] border border-[var(--color-accent-300)] text-[var(--color-accent-700)]",
        "hover:bg-[var(--color-accent-100)] hover:border-[var(--color-accent-500)]",
        "disabled:opacity-50 disabled:cursor-wait",
      )}
    >
      {busy ? <Loader2 size={13} className="animate-spin" /> : <Check size={13} strokeWidth={2} />}
      Mark step complete
    </button>
  );
}
