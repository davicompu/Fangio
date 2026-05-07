import { motion } from "framer-motion";
import { cn } from "../lib/utils";

export type TabStatus = "done" | "active" | "todo";

export interface ProgressionTab<Id extends string> {
  id: Id;
  label: string;
  icon: React.ComponentType<{ size?: number; strokeWidth?: number; className?: string }>;
  status: TabStatus;
  number: number; // 1-based
}

interface Props<Id extends string> {
  tabs: ProgressionTab<Id>[];
  onSelect: (id: Id) => void;
  underlineLayoutId: string;
}

export function ProgressionTabStrip<Id extends string>({ tabs, onSelect, underlineLayoutId }: Props<Id>) {
  return (
    <div className="flex items-end gap-1 border-b border-[var(--color-paper-300)]">
      {tabs.map((t) => {
        const Icon = t.icon;
        return (
          <button
            key={t.id}
            onClick={() => onSelect(t.id)}
            className={cn(
              "no-drag relative px-5 pt-3 pb-3.5 text-[13px] font-medium",
              "transition-paper flex items-center gap-2",
              t.status === "active"
                ? "text-[var(--color-ink-900)]"
                : t.status === "done"
                ? "text-[var(--color-paper-700)] hover:text-[var(--color-ink-700)]"
                : "text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)]",
            )}
          >
            <Badge status={t.status} number={t.number} />
            <Icon size={14} strokeWidth={1.75} />
            {t.label}
            {t.status === "active" && (
              <motion.div
                layoutId={underlineLayoutId}
                className="absolute left-0 right-0 -bottom-px h-0.5 bg-[var(--color-ink-700)]"
                transition={{ type: "spring", stiffness: 500, damping: 35 }}
              />
            )}
          </button>
        );
      })}
    </div>
  );
}

function Badge({ status, number }: { status: TabStatus; number: number }) {
  if (status === "done") {
    return (
      <span
        className="inline-flex items-center justify-center w-[18px] h-[18px] rounded-full bg-[var(--color-accent-500)] border border-[var(--color-accent-500)]"
        aria-label="Completed"
      >
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <path
            d="M1.5 5L4 7.5L8.5 2.5"
            stroke="white"
            strokeWidth="1.6"
            fill="none"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </span>
    );
  }
  if (status === "active") {
    return (
      <span
        className="inline-flex items-center justify-center w-[18px] h-[18px] rounded-full bg-[var(--color-ink-900)] text-white font-mono text-[10px] font-bold"
        aria-label={`Step ${number} (current)`}
      >
        {number}
      </span>
    );
  }
  return (
    <span
      className="inline-flex items-center justify-center w-[18px] h-[18px] rounded-full bg-[var(--color-accent-50)] border border-[var(--color-accent-300)] text-[var(--color-paper-500)] font-mono text-[10px] font-bold"
      aria-label={`Step ${number}`}
    >
      {number}
    </span>
  );
}
