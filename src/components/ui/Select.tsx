import { forwardRef, type SelectHTMLAttributes } from "react";
import { ChevronDown } from "lucide-react";
import { cn } from "../../lib/utils";

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  hint?: string;
}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(
  ({ label, hint, className, id, children, ...rest }, ref) => {
    const selectId = id || label?.toLowerCase().replace(/\s+/g, "-");
    return (
      <div className="flex flex-col gap-1.5">
        {label && (
          <label htmlFor={selectId} className="eyebrow text-[var(--color-paper-600)]">
            {label}
          </label>
        )}
        <div className="relative">
          <select
            ref={ref}
            id={selectId}
            className={cn(
              "appearance-none w-full h-11 pl-3.5 pr-10 rounded-[var(--radius-sm)]",
              "bg-[var(--color-paper-50)] border border-[var(--color-paper-300)]",
              "text-[var(--color-ink-800)] transition-paper",
              "focus:border-[var(--color-ink-600)] focus:bg-white focus:outline-none",
              className,
            )}
            {...rest}
          >
            {children}
          </select>
          <ChevronDown
            className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-[var(--color-paper-500)]"
            size={16}
            strokeWidth={1.75}
          />
        </div>
        {hint && <span className="text-xs text-[var(--color-paper-500)]">{hint}</span>}
      </div>
    );
  },
);
Select.displayName = "Select";
