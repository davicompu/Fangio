import { useCallback, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Bot } from "lucide-react";
import { api } from "../lib/api";
import { Button } from "./ui/Button";

type OpenTab = (tab: "screening" | "fulltext") => void;

interface Pending {
  context: string;
  auto: number;
  pending: number;
  target: "screening" | "fulltext";
  proceed: () => void | Promise<void>;
  onOpen: OpenTab;
}

/** Non-blocking reminder about AI screening decisions that were applied
 *  automatically and never opened, or are still pending. Never disables
 *  anything: "Continue anyway" always proceeds, and a failed fetch or log
 *  falls through to proceed. */
export function useAiReviewReminder() {
  const [state, setState] = useState<Pending | null>(null);
  const busy = useRef(false);

  const guard = useCallback(
    async (context: string, proceed: () => void | Promise<void>, onOpen: OpenTab) => {
      let counts;
      try {
        counts = await api.aiScreeningAttentionCounts();
      } catch {
        await proceed();
        return;
      }
      const auto = counts.title_abstract_auto + counts.fulltext_auto;
      const pending = counts.title_abstract_pending + counts.fulltext_pending;
      if (auto + pending === 0) {
        await proceed();
        return;
      }
      const hasTa = counts.title_abstract_auto + counts.title_abstract_pending > 0;
      setState({ context, auto, pending, target: hasTa ? "screening" : "fulltext", proceed, onOpen });
    },
    [],
  );

  const choose = async (choice: "open_now" | "continue") => {
    if (!state || busy.current) return;
    busy.current = true;
    const s = state;
    setState(null);
    try {
      await api.logAiReminderChoice(s.context, choice, s.auto, s.pending);
    } catch {
      /* logging must never block */
    }
    try {
      if (choice === "open_now") s.onOpen(s.target);
      else await s.proceed();
    } finally {
      busy.current = false;
    }
  };

  const element = (
    <AnimatePresence>
      {state && (
        <motion.div
          key="ai-reminder"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.12 }}
          className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/30 backdrop-blur-sm"
          onClick={() => setState(null)}
          onKeyDown={(e) => e.key === "Escape" && setState(null)}
        >
          <motion.div
            role="dialog"
            aria-modal="true"
            initial={{ opacity: 0, scale: 0.97, y: 6 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.97, y: 6 }}
            transition={{ duration: 0.15 }}
            className="w-full max-w-md bg-white rounded-[var(--radius-lg)] border border-[var(--color-paper-300)] shadow-[var(--shadow-paper-lg)] overflow-hidden"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-2 px-5 py-3.5 border-b border-[var(--color-paper-200)]">
              <Bot size={14} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
              <span className="text-[13px] font-semibold text-[var(--color-ink-900)]">AI screening decisions</span>
            </div>
            <p className="px-5 py-4 text-[13px] leading-relaxed text-[var(--color-ink-800)]">
              {state.auto} AI screening decisions were applied automatically and haven't been opened; {state.pending} are pending.
            </p>
            <div className="flex justify-end gap-2 px-5 py-3 border-t border-[var(--color-paper-200)]">
              <Button variant="secondary" size="sm" onClick={() => choose("continue")}>
                Continue anyway
              </Button>
              <Button variant="primary" size="sm" autoFocus onClick={() => choose("open_now")}>
                Open them now
              </Button>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );

  return { guard, element };
}
