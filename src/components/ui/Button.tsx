import { forwardRef, type ButtonHTMLAttributes } from "react";
import { cn } from "../../lib/utils";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const variantClasses: Record<Variant, string> = {
  primary:
    "bg-[var(--color-ink-800)] text-[var(--color-paper-50)] hover:bg-[var(--color-ink-700)] active:bg-[var(--color-ink-900)] disabled:bg-[var(--color-paper-300)] disabled:text-[var(--color-paper-500)]",
  secondary:
    "bg-[var(--color-paper-100)] text-[var(--color-ink-800)] border border-[var(--color-paper-300)] hover:bg-[var(--color-paper-200)] hover:border-[var(--color-paper-400)] active:bg-[var(--color-paper-300)] disabled:opacity-50",
  ghost:
    "bg-transparent text-[var(--color-ink-700)] hover:bg-[var(--color-paper-100)] active:bg-[var(--color-paper-200)] disabled:opacity-50",
  danger:
    "bg-[var(--color-error-500)] text-[var(--color-paper-50)] hover:opacity-90 active:opacity-80",
};

const sizeClasses: Record<Size, string> = {
  sm: "h-8 px-3 text-xs gap-1.5",
  md: "h-10 px-4 text-sm gap-2",
  lg: "h-12 px-6 text-[15px] gap-2.5",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = "primary", size = "md", className, children, ...rest }, ref) => {
    return (
      <button
        ref={ref}
        className={cn(
          "inline-flex items-center justify-center rounded-[var(--radius-sm)] font-medium tracking-tight",
          "transition-paper select-none",
          "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-ink-700)]",
          variantClasses[variant],
          sizeClasses[size],
          className,
        )}
        {...rest}
      >
        {children}
      </button>
    );
  },
);
Button.displayName = "Button";
