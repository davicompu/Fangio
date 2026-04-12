import { useMemo } from "react";
import { marked } from "marked";

// Configure marked for safe, clean output
marked.setOptions({ breaks: true });

interface Props {
  children: string;
  className?: string;
}

/**
 * Renders a markdown string as sanitised HTML.
 * Applies prose-style typography via the `markdown-body` class.
 */
export function Markdown({ children, className }: Props) {
  const html = useMemo(() => marked.parse(children || "") as string, [children]);
  return (
    <div
      className={["markdown-body", className].filter(Boolean).join(" ")}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
