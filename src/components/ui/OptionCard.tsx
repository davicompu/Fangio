import { type ReactNode } from "react";
import { Check } from "lucide-react";
import { cn } from "../../lib/utils";

interface OptionCardProps {
  selected?: boolean;
  onSelect?: () => void;
  title: string;
  description?: string;
  badge?: string;
  icon?: ReactNode;
  recommended?: boolean;
}

/**
 * A large, clickable choice. Used in onboarding for SLR type / framework
 * selection. Designed to feel weighty rather than form-y — radio buttons by
 * any other name.
 */
export function OptionCard({
  selected,
  onSelect,
  title,
  description,
  badge,
  icon,
  recommended,
}: OptionCardProps) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        "group text-left w-full p-5 rounded-[var(--radius-md)]",
        "bg-[var(--color-paper-50)] border transition-paper",
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-ink-700)]",
        selected
          ? "border-[var(--color-ink-700)] shadow-[var(--shadow-paper-md)] bg-white"
          : "border-[var(--color-paper-300)] hover:border-[var(--color-paper-500)] hover:bg-white",
      )}
    >
      <div className="flex items-start gap-4">
        {icon && (
          <div
            className={cn(
              "shrink-0 mt-0.5 w-9 h-9 rounded-[var(--radius-sm)] flex items-center justify-center",
              "transition-paper",
              selected
                ? "bg-[var(--color-ink-700)] text-[var(--color-paper-50)]"
                : "bg-[var(--color-paper-200)] text-[var(--color-ink-700)] group-hover:bg-[var(--color-paper-300)]",
            )}
          >
            {icon}
          </div>
        )}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <h3 className="font-semibold text-[15px] tracking-tight text-[var(--color-ink-900)]">
              {title}
            </h3>
            {recommended && (
              <span className="text-[10px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded-sm bg-[var(--color-accent-100)] text-[var(--color-accent-700)]">
                Recommended
              </span>
            )}
            {badge && (
              <span className="text-[10px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded-sm bg-[var(--color-paper-200)] text-[var(--color-paper-600)]">
                {badge}
              </span>
            )}
          </div>
          {description && (
            <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed">
              {description}
            </p>
          )}
        </div>
        <div
          className={cn(
            "shrink-0 mt-1 w-5 h-5 rounded-full border-2 transition-paper flex items-center justify-center",
            selected
              ? "border-[var(--color-ink-700)] bg-[var(--color-ink-700)]"
              : "border-[var(--color-paper-400)] group-hover:border-[var(--color-paper-500)]",
          )}
        >
          {selected && <Check size={12} strokeWidth={3} className="text-[var(--color-paper-50)]" />}
        </div>
      </div>
    </button>
  );
}
