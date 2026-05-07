import { useState } from "react";
import { Check } from "lucide-react";
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
      <div className="mt-8 pt-4 border-t border-[var(--color-paper-200)] flex items-center gap-3 text-[12px]">
        <span className="inline-flex items-center gap-1.5 text-[var(--color-ok-600)]">
          <Check size={13} strokeWidth={2} />
          Marked complete
        </span>
        <button
          type="button"
          onClick={() => setFlag(false)}
          disabled={busy}
          className={cn(
            "text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] underline-offset-2 hover:underline transition-paper",
            busy && "opacity-50 cursor-wait",
          )}
        >
          Undo
        </button>
      </div>
    );
  }

  return (
    <div className="mt-8 pt-4 border-t border-[var(--color-paper-200)]">
      <button
        type="button"
        onClick={() => setFlag(true)}
        disabled={busy}
        className={cn(
          "text-[12px] text-[var(--color-ink-700)] hover:text-[var(--color-accent-600)] underline-offset-2 hover:underline transition-paper",
          busy && "opacity-50 cursor-wait",
        )}
      >
        Mark this step complete →
      </button>
    </div>
  );
}
