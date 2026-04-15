import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  Check,
  ChevronDown,
  ChevronUp,
  Loader2,
  Pause,
  Play,
  Sparkles,
  X,
} from "lucide-react";
import { api, type AiCodingCoverage, type DimensionInfo, type LmModel } from "../lib/api";
import type { AiCodingHook } from "../hooks/useAiCoding";
import { cn } from "../lib/utils";
import { Button } from "./ui/Button";

interface Props {
  open: boolean;
  onClose: () => void;
  coding: AiCodingHook;
  availableModels: LmModel[];
  availableDimensions: DimensionInfo[];
  defaultChatModel: string;
  onSendToBackground: () => void;
}

const PHASE_LABELS: Record<string, string> = {
  "indexing": "Indexing PDF",
  "coding-rater1": "Rater 1 coding",
  "coding-rater2": "Rater 2 coding",
  "resolving": "Resolving",
};

export function AiCodingModal({
  open,
  onClose,
  coding,
  availableModels,
  availableDimensions,
  defaultChatModel,
  onSendToBackground,
}: Props) {
  const [rater1Model, setRater1Model] = useState(defaultChatModel);
  const [rater2Model, setRater2Model] = useState(defaultChatModel);
  const [dualRater, setDualRater] = useState(true);
  const [selectedDims, setSelectedDims] = useState<Set<string>>(
    new Set(availableDimensions.map((d) => d.name)),
  );
  const [coverage, setCoverage] = useState<AiCodingCoverage | null>(null);
  const [loadingCoverage, setLoadingCoverage] = useState(false);

  // Load coverage when the modal opens in idle state
  useEffect(() => {
    if (!open || coding.step !== "idle") return;
    setLoadingCoverage(true);
    api.getAiCodingCoverage()
      .then(setCoverage)
      .catch(() => setCoverage(null))
      .finally(() => setLoadingCoverage(false));
  }, [open, coding.step]);

  // Keep selectedDims in sync if dimensions change
  useEffect(() => {
    setSelectedDims(new Set(availableDimensions.map((d) => d.name)));
  }, [availableDimensions]);

  const toggleDim = (name: string) => {
    setSelectedDims((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name); else next.add(name);
      return next;
    });
  };

  const handleStart = () => {
    if (selectedDims.size === 0) return;
    coding.start({
      rater1Model,
      rater2Model: dualRater ? rater2Model : null,
      dimensions: [...selectedDims],
    });
  };

  if (!open) return null;

  const progress = coding.progress;
  const pct = progress && progress.total > 0
    ? Math.round((progress.done / progress.total) * 100)
    : 0;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/40 backdrop-blur-sm">
      <motion.div
        initial={{ opacity: 0, scale: 0.97, y: 8 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.97, y: 8 }}
        transition={{ duration: 0.18 }}
        className="w-full max-w-lg bg-white rounded-[var(--radius-lg)] border border-[var(--color-paper-300)] shadow-[var(--shadow-paper-lg)] overflow-hidden"
      >
        {/* Header */}
        <div className="flex items-center gap-2.5 px-5 py-4 border-b border-[var(--color-paper-200)]">
          <Sparkles size={16} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
          <h2 className="font-semibold text-[15px] text-[var(--color-ink-900)] flex-1">
            Code with AI
          </h2>
          <button
            onClick={onClose}
            className="p-1 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] text-[var(--color-paper-500)] transition-paper"
          >
            <X size={14} strokeWidth={2} />
          </button>
        </div>

        {/* Body */}
        <div className="px-5 py-4 space-y-5">

          {/* ── Config form (idle) ── */}
          {coding.step === "idle" && (
            <>
              {/* Coverage summary */}
              <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-200)] bg-[var(--color-paper-50)] px-4 py-3 space-y-1.5">
                <p className="text-[11px] font-semibold uppercase tracking-wider text-[var(--color-paper-500)] mb-2">
                  Coverage
                </p>
                {loadingCoverage && (
                  <div className="flex items-center gap-2 text-[12px] text-[var(--color-paper-500)]">
                    <Loader2 size={12} className="animate-spin" /> Loading…
                  </div>
                )}
                {coverage && !loadingCoverage && (
                  <>
                    <div className="flex items-center gap-2 text-[12px] text-[var(--color-ok-700)]">
                      <Check size={12} strokeWidth={2.5} />
                      <span>{coverage.ready} article{coverage.ready !== 1 ? "s" : ""} ready to code</span>
                    </div>
                    {coverage.needs_indexing > 0 && (
                      <p className="text-[12px] text-[var(--color-warn-700)]">
                        ↻ {coverage.needs_indexing} article{coverage.needs_indexing !== 1 ? "s" : ""} need indexing — will be done automatically
                      </p>
                    )}
                    {coverage.no_pdf > 0 && (
                      <p className="text-[12px] text-[var(--color-paper-500)]">
                        ⊘ {coverage.no_pdf} article{coverage.no_pdf !== 1 ? "s" : ""} missing PDF — will be skipped
                      </p>
                    )}
                  </>
                )}
              </div>

              {/* Dimensions */}
              <div>
                <p className="text-[11px] font-semibold uppercase tracking-wider text-[var(--color-paper-500)] mb-2">
                  Dimensions
                </p>
                <div className="flex flex-wrap gap-1.5">
                  {availableDimensions.map((d) => (
                    <button
                      key={d.name}
                      onClick={() => toggleDim(d.name)}
                      className={cn(
                        "px-2.5 py-1 rounded-full text-[11px] font-medium border transition-paper",
                        selectedDims.has(d.name)
                          ? "bg-[var(--color-ink-800)] border-[var(--color-ink-800)] text-white"
                          : "bg-white border-[var(--color-paper-300)] text-[var(--color-paper-600)] hover:border-[var(--color-ink-400)]",
                      )}
                    >
                      {d.name}
                    </button>
                  ))}
                </div>
              </div>

              {/* Raters */}
              <div>
                <p className="text-[11px] font-semibold uppercase tracking-wider text-[var(--color-paper-500)] mb-2">
                  Raters
                </p>

                <div className="space-y-2">
                  {/* Rater 1 */}
                  <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-200)] px-3 py-2.5">
                    <p className="text-[11px] font-semibold text-[var(--color-paper-500)] mb-1.5">
                      Rater 1
                    </p>
                    <select
                      value={rater1Model}
                      onChange={(e) => setRater1Model(e.target.value)}
                      className="w-full text-[12px] px-2.5 py-1.5 rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white text-[var(--color-ink-800)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-400)] transition-paper"
                    >
                      {availableModels.length === 0 && (
                        <option value={rater1Model}>{rater1Model}</option>
                      )}
                      {availableModels.map((m) => (
                        <option key={m.id} value={m.id}>{m.id}</option>
                      ))}
                    </select>
                  </div>

                  {/* Rater 2 toggle */}
                  <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-200)] overflow-hidden">
                    <button
                      onClick={() => setDualRater((v) => !v)}
                      className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-[var(--color-paper-50)] transition-paper"
                    >
                      <div>
                        <p className="text-[11px] font-semibold text-[var(--color-paper-500)]">
                          Rater 2 {dualRater ? "" : "(disabled)"}
                        </p>
                        {!dualRater && (
                          <p className="text-[10px] text-[var(--color-paper-400)] mt-0.5">
                            All codes will be applied automatically
                          </p>
                        )}
                      </div>
                      {dualRater
                        ? <ChevronUp size={13} className="text-[var(--color-paper-400)]" />
                        : <ChevronDown size={13} className="text-[var(--color-paper-400)]" />
                      }
                    </button>
                    <AnimatePresence>
                      {dualRater && (
                        <motion.div
                          initial={{ height: 0, opacity: 0 }}
                          animate={{ height: "auto", opacity: 1 }}
                          exit={{ height: 0, opacity: 0 }}
                          transition={{ duration: 0.15 }}
                          className="overflow-hidden border-t border-[var(--color-paper-200)] px-3 py-2.5"
                        >
                          <select
                            value={rater2Model}
                            onChange={(e) => setRater2Model(e.target.value)}
                            className="w-full text-[12px] px-2.5 py-1.5 rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white text-[var(--color-ink-800)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-400)] transition-paper"
                          >
                            {availableModels.length === 0 && (
                              <option value={rater2Model}>{rater2Model}</option>
                            )}
                            {availableModels.map((m) => (
                              <option key={m.id} value={m.id}>{m.id}</option>
                            ))}
                          </select>
                          <p className="mt-1.5 text-[10px] text-[var(--color-paper-500)] leading-relaxed">
                            Agreed codes are applied automatically. Disagreements are flagged for your review.
                          </p>
                        </motion.div>
                      )}
                    </AnimatePresence>
                  </div>
                </div>
              </div>
            </>
          )}

          {/* ── Running ── */}
          {coding.step === "running" && (
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                {coding.isPaused
                  ? <Pause size={14} strokeWidth={2} className="text-[var(--color-warn-500)]" />
                  : <Loader2 size={14} className="animate-spin text-[var(--color-accent-500)]" />
                }
                <span className="text-[13px] font-medium text-[var(--color-ink-800)]">
                  {coding.isPaused ? "Paused" : (progress ? (PHASE_LABELS[progress.phase] ?? "Coding") : "Starting…")}
                </span>
              </div>

              {progress && (
                <>
                  <p className="text-[12px] text-[var(--color-paper-600)] truncate">
                    {progress.article_title}
                  </p>
                  <div className="space-y-1">
                    <div className="flex justify-between text-[11px] text-[var(--color-paper-500)]">
                      <span>{progress.done} / {progress.total} articles</span>
                      <span>{pct}%</span>
                    </div>
                    <div className="h-1.5 rounded-full bg-[var(--color-paper-200)] overflow-hidden">
                      <motion.div
                        animate={{ width: `${pct}%` }}
                        transition={{ duration: 0.3 }}
                        className="h-full rounded-full bg-[var(--color-accent-500)]"
                      />
                    </div>
                  </div>
                  <div className="flex gap-4 text-[11px] text-[var(--color-paper-500)]">
                    {progress.consensus > 0 && (
                      <span className="text-[var(--color-ok-600)]">✓ {progress.consensus} consensus</span>
                    )}
                    {progress.disagreements > 0 && (
                      <span className="text-amber-600">⚠ {progress.disagreements} need review</span>
                    )}
                    {progress.skipped_no_pdf > 0 && (
                      <span>⊘ {progress.skipped_no_pdf} skipped</span>
                    )}
                  </div>
                </>
              )}
            </div>
          )}

          {/* ── Done ── */}
          {coding.step === "done" && coding.result && (
            <div className="space-y-2 text-[13px]">
              <div className="flex items-center gap-2 text-[var(--color-ok-700)]">
                <Check size={16} strokeWidth={2.5} />
                <span className="font-semibold">Coding complete</span>
              </div>
              <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-200)] divide-y divide-[var(--color-paper-200)]">
                <div className="flex justify-between px-3 py-2 text-[12px]">
                  <span className="text-[var(--color-paper-600)]">Articles processed</span>
                  <span className="font-medium">{coding.result.processed}</span>
                </div>
                <div className="flex justify-between px-3 py-2 text-[12px]">
                  <span className="text-[var(--color-ok-600)]">Consensus codes applied</span>
                  <span className="font-medium text-[var(--color-ok-700)]">{coding.result.consensus}</span>
                </div>
                {coding.result.disagreements > 0 && (
                  <div className="flex justify-between px-3 py-2 text-[12px]">
                    <span className="text-amber-600">Disagreements to review</span>
                    <span className="font-medium text-amber-700">{coding.result.disagreements}</span>
                  </div>
                )}
                {coding.result.skipped_no_pdf > 0 && (
                  <div className="flex justify-between px-3 py-2 text-[12px]">
                    <span className="text-[var(--color-paper-500)]">Skipped (no PDF)</span>
                    <span className="font-medium">{coding.result.skipped_no_pdf}</span>
                  </div>
                )}
                {coding.result.indexed > 0 && (
                  <div className="flex justify-between px-3 py-2 text-[12px]">
                    <span className="text-[var(--color-paper-500)]">PDFs indexed during run</span>
                    <span className="font-medium">{coding.result.indexed}</span>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* ── Error ── */}
          {coding.step === "error" && (
            <p className="text-[12px] text-[var(--color-error-600)] leading-relaxed">
              {coding.error ?? "An unexpected error occurred."}
            </p>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between gap-2 px-5 py-3 border-t border-[var(--color-paper-200)] bg-[var(--color-paper-50)]">
          {coding.step === "idle" && (
            <>
              <Button variant="ghost" size="sm" onClick={onClose}>Cancel</Button>
              <Button
                variant="primary"
                size="sm"
                onClick={handleStart}
                disabled={selectedDims.size === 0 || (coverage !== null && coverage.ready === 0 && coverage.needs_indexing === 0)}
              >
                <Sparkles size={13} strokeWidth={1.75} />
                Run in background
              </Button>
            </>
          )}

          {coding.step === "running" && (
            <>
              <button
                onClick={() => coding.isPaused ? coding.resume() : coding.pause()}
                className="flex items-center gap-1.5 text-[12px] text-[var(--color-paper-600)] hover:text-[var(--color-ink-800)] transition-paper"
              >
                {coding.isPaused
                  ? <><Play size={12} strokeWidth={2} /> Resume</>
                  : <><Pause size={12} strokeWidth={2} /> Pause</>
                }
              </button>
              <div className="flex items-center gap-2">
                <button
                  onClick={onSendToBackground}
                  className="text-[12px] text-[var(--color-accent-600)] hover:text-[var(--color-accent-700)] font-medium transition-paper"
                >
                  Send to background
                </button>
                <button
                  onClick={() => coding.cancel()}
                  className="text-[12px] text-[var(--color-error-500)] hover:text-[var(--color-error-700)] transition-paper"
                >
                  Cancel
                </button>
              </div>
            </>
          )}

          {(coding.step === "done" || coding.step === "error") && (
            <>
              <span />
              <Button variant="primary" size="sm" onClick={onClose}>
                Done
              </Button>
            </>
          )}
        </div>
      </motion.div>
    </div>
  );
}
