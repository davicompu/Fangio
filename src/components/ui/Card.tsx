import { type HTMLAttributes } from "react";
import { cn } from "../../lib/utils";

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  hoverable?: boolean;
}

export function Card({ className, hoverable, ...rest }: CardProps) {
  return (
    <div
      className={cn(
        "bg-[var(--color-paper-50)] border border-[var(--color-paper-200)]",
        "rounded-[var(--radius-md)] shadow-[var(--shadow-paper)]",
        "transition-paper",
        hoverable &&
          "hover:border-[var(--color-paper-400)] hover:shadow-[var(--shadow-paper-md)] hover:-translate-y-0.5 cursor-default",
        className,
      )}
      {...rest}
    />
  );
}
