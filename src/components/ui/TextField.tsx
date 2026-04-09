import { forwardRef, type InputHTMLAttributes, type TextareaHTMLAttributes } from "react";
import { cn } from "../../lib/utils";

interface TextFieldProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  hint?: string;
  error?: string;
  mono?: boolean;
}

export const TextField = forwardRef<HTMLInputElement, TextFieldProps>(
  ({ label, hint, error, mono, className, id, ...rest }, ref) => {
    const inputId = id || label?.toLowerCase().replace(/\s+/g, "-");
    return (
      <div className="flex flex-col gap-1.5">
        {label && (
          <label htmlFor={inputId} className="eyebrow text-[var(--color-paper-600)]">
            {label}
          </label>
        )}
        <input
          ref={ref}
          id={inputId}
          className={cn(
            "h-11 px-3.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-50)]",
            "border border-[var(--color-paper-300)] text-[var(--color-ink-800)]",
            "placeholder:text-[var(--color-paper-400)]",
            "transition-paper focus:border-[var(--color-ink-600)] focus:bg-white",
            "focus:outline-none focus-visible:ring-0",
            mono && "font-mono text-[13px]",
            error && "border-[var(--color-error-500)]",
            className,
          )}
          {...rest}
        />
        {error ? (
          <span className="text-xs text-[var(--color-error-500)]">{error}</span>
        ) : hint ? (
          <span className="text-xs text-[var(--color-paper-500)]">{hint}</span>
        ) : null}
      </div>
    );
  },
);
TextField.displayName = "TextField";

interface TextAreaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  hint?: string;
  error?: string;
}

export const TextArea = forwardRef<HTMLTextAreaElement, TextAreaProps>(
  ({ label, hint, error, className, id, rows = 4, ...rest }, ref) => {
    const inputId = id || label?.toLowerCase().replace(/\s+/g, "-");
    return (
      <div className="flex flex-col gap-1.5">
        {label && (
          <label htmlFor={inputId} className="eyebrow text-[var(--color-paper-600)]">
            {label}
          </label>
        )}
        <textarea
          ref={ref}
          id={inputId}
          rows={rows}
          className={cn(
            "px-3.5 py-3 rounded-[var(--radius-sm)] bg-[var(--color-paper-50)]",
            "border border-[var(--color-paper-300)] text-[var(--color-ink-800)]",
            "placeholder:text-[var(--color-paper-400)] leading-relaxed",
            "transition-paper focus:border-[var(--color-ink-600)] focus:bg-white",
            "focus:outline-none focus-visible:ring-0",
            "resize-y min-h-[88px]",
            error && "border-[var(--color-error-500)]",
            className,
          )}
          {...rest}
        />
        {error ? (
          <span className="text-xs text-[var(--color-error-500)]">{error}</span>
        ) : hint ? (
          <span className="text-xs text-[var(--color-paper-500)]">{hint}</span>
        ) : null}
      </div>
    );
  },
);
TextArea.displayName = "TextArea";
