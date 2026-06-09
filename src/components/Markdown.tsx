import { useMemo } from "react";
import { marked } from "marked";
import DOMPurify from "dompurify";

marked.setOptions({ breaks: true });

interface Props {
  children: string;
  className?: string;
}

/**
 * Renders a markdown string as sanitised HTML.
 * Applies prose-style typography via the `markdown-body` class.
 *
 * `marked` does NOT sanitise — it passes raw HTML through — so its output is
 * run through DOMPurify before injection. This content includes LLM answers
 * derived from imported PDFs, which can carry attacker-authored markup.
 */
export function Markdown({ children, className }: Props) {
  const html = useMemo(
    () => DOMPurify.sanitize(marked.parse(children || "") as string),
    [children],
  );
  return (
    <div
      className={["markdown-body", className].filter(Boolean).join(" ")}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
