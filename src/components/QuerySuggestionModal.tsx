import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Check, Clipboard, Loader2, Sparkles, X } from "lucide-react";
import { Button } from "./ui/Button";
import { Card } from "./ui/Card";
import type { SuggestQueryResult } from "../lib/api";

interface Props {
  open: boolean;
  mode: "suggest" | "advise";
  result: SuggestQueryResult | null;
  error: string | null;
  onAccept: (query: string) => void;
  onDismiss: () => void;
}

export function QuerySuggestionModal({ open, mode, result, error, onAccept, onDismiss }: Props) {
  const [copied, setCopied] = useState(false);

  // Close on Escape
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onDismiss();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onDismiss]);

  // Reset the copied indicator when the modal closes or the result changes.
  useEffect(() => {
    if (!open) setCopied(false);
  }, [open, result]);

  const handleCopy = async () => {
    if (!result?.query) return;
    try {
      await navigator.clipboard.writeText(result.query);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // ignore — older WebKit may block; user can fall back to manual copy
    }
  };

  return (
    <AnimatePresence>
      {open && (
        <>
          {/* Backdrop */}
          <motion.div
            key="backdrop"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="fixed inset-0 bg-[var(--color-ink-900)]/30 backdrop-blur-sm z-40"
            onClick={onDismiss}
          />

          {/* Modal */}
          <motion.div
            key="modal"
            initial={{ opacity: 0, scale: 0.96, y: 8 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: 8 }}
            transition={{ duration: 0.2, ease: [0.2, 0.7, 0.2, 1] }}
            className="fixed inset-0 z-50 flex items-center justify-center p-6 pointer-events-none"
          >
            <Card
              role="dialog"
              aria-modal="true"
              aria-label={mode === "suggest" ? "AI Suggestion" : "AI Advisement"}
              className="pointer-events-auto w-full max-w-2xl p-6 shadow-[var(--shadow-paper)]"
            >
              {/* Header */}
              <div className="flex items-start justify-between mb-4">
                <div className="flex items-center gap-2">
                  <Sparkles size={15} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                  <span className="eyebrow">
                    {mode === "suggest" ? "AI Suggestion" : "AI Advisement"}
                  </span>
                </div>
                <button
                  onClick={onDismiss}
                  className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
                  aria-label="Dismiss"
                >
                  <X size={14} strokeWidth={1.75} />
                </button>
              </div>

              {/* Loading state */}
              {!result && !error && (
                <div className="flex items-center gap-3 py-8 justify-center text-[var(--color-paper-500)]">
                  <Loader2 size={18} className="animate-spin" />
                  <span className="text-[14px]">Thinking…</span>
                </div>
              )}

              {/* Error state */}
              {error && (
                <div>
                  <div className="px-4 py-3 rounded-[var(--radius-sm)] bg-[var(--color-accent-50)] border border-[var(--color-accent-300)] text-[13px] text-[var(--color-accent-700)]">
                    {error}
                  </div>
                  <div className="mt-4">
                    <Button variant="ghost" size="sm" onClick={onDismiss}>
                      Dismiss
                    </Button>
                  </div>
                </div>
              )}

              {/* Result state */}
              {result && (
                <>
                  <pre className="font-mono text-[12px] leading-relaxed whitespace-pre-wrap bg-[var(--color-paper-100)] border border-[var(--color-paper-200)] rounded-[var(--radius-sm)] px-4 py-3 text-[var(--color-ink-800)] mb-3 max-h-60 overflow-y-auto">
                    {result.query}
                  </pre>
                  {result.rationale && (
                    <p className="text-[12px] text-[var(--color-paper-600)] leading-relaxed mb-4">
                      {result.rationale}
                    </p>
                  )}
                  <div className="flex items-center gap-3">
                    <Button variant="primary" size="sm" onClick={() => onAccept(result.query)}>
                      Accept
                    </Button>
                    <Button variant="secondary" size="sm" onClick={handleCopy}>
                      {copied ? (
                        <>
                          <Check size={13} strokeWidth={2} className="text-[var(--color-ok-600)]" />
                          Copied
                        </>
                      ) : (
                        <>
                          <Clipboard size={13} strokeWidth={2} />
                          Copy
                        </>
                      )}
                    </Button>
                    <Button variant="ghost" size="sm" onClick={onDismiss}>
                      Dismiss
                    </Button>
                  </div>
                </>
              )}


            </Card>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
