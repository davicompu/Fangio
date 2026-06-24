import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Bot, ChevronRight, Loader2, Pause, Play, Plus, Sparkles, X } from "lucide-react";
import { type AiScreeningProgress, type AiScreeningResult, type ExclusionCriterionRow, type ProjectRow } from "../lib/api";
import { Button } from "./ui/Button";
import { Card } from "./ui/Card";
import { cn } from "../lib/utils";

export interface AiScreeningHook {
  step: "idle" | "running" | "done" | "error";
  progress: AiScreeningProgress | null;
  result: AiScreeningResult | null;
  error: string | null;
  isPaused: boolean;
  start: (exclusionCriteria: string[], excludeOnly?: boolean) => Promise<void>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  cancel: () => Promise<void>;
  reset: () => void;
}

interface Props {
  open: boolean;
  project: ProjectRow;
  criteria: ExclusionCriterionRow[];
  screening: AiScreeningHook;
  onComplete: (result: AiScreeningResult) => void;
  onDismiss: () => void;
  onSendToBackground: () => void;
}

type LocalStep = "checklist" | "extra";

// Standard PRISMA criteria always shown in the checklist.
const STANDARD_CRITERIA = [
  { number: 2, label: "Outside search period" },
  { number: 3, label: "Below source quality threshold" },
  { number: 4, label: "Wrong source type (e.g. conference proceedings, book chapters)" },
  { number: 5, label: "Non-English publication" },
  { number: 6, label: "Predatory journal" },
  { number: 7, label: "Full text not accessible" },
  { number: 8, label: "Not directly relevant to domain" },
  { number: 9, label: "Retracted article" },
];

export function AiScreeningModal({ open, project, criteria: _criteria, screening, onComplete, onDismiss, onSendToBackground }: Props) {
  // Local steps only used while screening.step === "idle"
  const [localStep, setLocalStep] = useState<LocalStep>("checklist");
  const [selectedEc, setSelectedEc] = useState<Set<number>>(new Set());
  const [wantsExtra, setWantsExtra] = useState<boolean | null>(null);
  const [extraInput, setExtraInput] = useState("");

  const domain = project.domain ?? null;

  // Reset local state whenever modal opens and screening is idle
  useEffect(() => {
    if (!open) return;
    if (screening.step === "idle") {
      setLocalStep("checklist");
      setSelectedEc(new Set(STANDARD_CRITERIA.map((c) => c.number)));
      setWantsExtra(null);
      setExtraInput("");
    }
  }, [open, screening.step]);

  // Escape key — only when not actively running
  useEffect(() => {
    if (!open || screening.step === "running") return;
    const handler = (e: KeyboardEvent) => { if (e.key === "Escape") onDismiss(); };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, screening.step, onDismiss]);

  const toggleEc = (n: number) => {
    setSelectedEc((prev) => {
      const next = new Set(prev);
      if (next.has(n)) next.delete(n); else next.add(n);
      return next;
    });
  };

  const handleExtraDecision = (yes: boolean) => {
    setWantsExtra(yes);
    if (!yes) startRun();
    else setLocalStep("extra");
  };

  const handleExtraSubmit = () => {
    startRun(extraInput.trim() || undefined);
  };

  const startRun = (extraCriterion?: string) => {
    const exclusionCriteria = STANDARD_CRITERIA
      .filter((c) => selectedEc.has(c.number))
      .map((c) => `EC${c.number}: ${c.label}`);
    if (extraCriterion) exclusionCriteria.push(extraCriterion);
    screening.start(exclusionCriteria);
  };

  const handlePauseResume = () => {
    if (screening.isPaused) screening.resume();
    else screening.pause();
  };

  // Derive which view to show
  const showScreeningView = screening.step !== "idle";

  return (
    <AnimatePresence>
      {open && (
        <>
          <motion.div
            key="backdrop"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="fixed inset-0 bg-[var(--color-ink-900)]/30 backdrop-blur-sm z-40"
            onClick={screening.step !== "running" ? onDismiss : undefined}
          />
          <motion.div
            key="modal"
            initial={{ opacity: 0, scale: 0.97, y: 8 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.97, y: 8 }}
            transition={{ duration: 0.2, ease: [0.2, 0.7, 0.2, 1] }}
            className="fixed inset-0 z-50 flex items-center justify-center p-6 pointer-events-none"
          >
            <Card
              role="dialog"
              aria-modal="true"
              className="pointer-events-auto w-full max-w-xl p-6 shadow-[var(--shadow-paper-md)]"
            >
              {/* Header */}
              <div className="flex items-start justify-between mb-5">
                <div className="flex items-center gap-2">
                  <Sparkles size={15} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                  <span className="eyebrow">Screen with AI</span>
                </div>
                {screening.step !== "running" && (
                  <button
                    onClick={onDismiss}
                    className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
                  >
                    <X size={14} strokeWidth={1.75} />
                  </button>
                )}
              </div>

              {/* ── Idle: checklist ── */}
              {!showScreeningView && localStep === "checklist" && (
                <div>
                  <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed mb-4">
                    The AI will judge relevance against your domain scope, then apply the
                    exclusion criteria below.
                  </p>

                  <div className="mb-4 px-3 py-2.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-200)]">
                    <p className="text-[11px] font-semibold uppercase tracking-wide text-[var(--color-paper-500)] mb-1">
                      Domain scope
                    </p>
                    {domain ? (
                      <p className="text-[12px] text-[var(--color-ink-700)] leading-relaxed">{domain}</p>
                    ) : (
                      <p className="text-[12px] text-[var(--color-paper-400)] italic">
                        Not set — add a domain scope in project settings for better results.
                      </p>
                    )}
                  </div>

                  <p className="text-[11px] font-semibold uppercase tracking-wide text-[var(--color-paper-500)] mb-2">
                    Exclusion criteria
                  </p>
                  <ul className="space-y-1.5 max-h-64 overflow-y-auto pr-1">
                    {STANDARD_CRITERIA.map((c) => (
                      <li key={c.number}>
                        <button
                          type="button"
                          onClick={() => toggleEc(c.number)}
                          className={cn(
                            "w-full text-left flex items-center gap-3 px-3 py-2 rounded-[var(--radius-sm)] border text-[12px] transition-paper",
                            selectedEc.has(c.number)
                              ? "bg-white border-[var(--color-ink-700)] text-[var(--color-ink-800)]"
                              : "bg-[var(--color-paper-50)] border-[var(--color-paper-300)] text-[var(--color-paper-500)]",
                          )}
                        >
                          <div
                            className={cn(
                              "w-4 h-4 rounded-[3px] border-2 flex items-center justify-center shrink-0 transition-paper",
                              selectedEc.has(c.number)
                                ? "bg-[var(--color-ink-700)] border-[var(--color-ink-700)]"
                                : "border-[var(--color-paper-400)]",
                            )}
                          >
                            {selectedEc.has(c.number) && (
                              <svg width="9" height="9" viewBox="0 0 9 9">
                                <path d="M1 4.5L3.5 7L8 1.5" stroke="white" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round" />
                              </svg>
                            )}
                          </div>
                          <span>
                            <span className="font-mono text-[10px] text-[var(--color-paper-400)] mr-1.5">EC{c.number}</span>
                            {c.label}
                          </span>
                        </button>
                      </li>
                    ))}
                  </ul>

                  <div className="mt-5 flex items-center justify-between">
                    <span className="text-[12px] text-[var(--color-paper-500)]">
                      {selectedEc.size} criteria selected
                    </span>
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={() => setLocalStep("extra")}
                    >
                      Continue
                      <ChevronRight size={13} strokeWidth={2} />
                    </Button>
                  </div>
                </div>
              )}

              {/* ── Idle: extra criteria — ask ── */}
              {!showScreeningView && localStep === "extra" && wantsExtra === null && (
                <div>
                  <p className="text-[14px] text-[var(--color-ink-800)] font-medium mb-2">
                    Any additional criteria?
                  </p>
                  <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed mb-5">
                    Add a domain-specific criterion the standard list doesn't cover.
                  </p>
                  <div className="flex gap-3">
                    <Button variant="primary" size="sm" onClick={() => handleExtraDecision(true)}>
                      <Plus size={13} strokeWidth={2} />
                      Yes, add one
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => handleExtraDecision(false)}>
                      No, begin screening
                      <Bot size={13} strokeWidth={1.75} />
                    </Button>
                  </div>
                </div>
              )}

              {/* ── Idle: extra criteria — input ── */}
              {!showScreeningView && localStep === "extra" && wantsExtra === true && (
                <div>
                  <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed mb-3">
                    Describe the additional criterion in plain language.
                  </p>
                  <textarea
                    autoFocus
                    value={extraInput}
                    onChange={(e) => setExtraInput(e.target.value)}
                    rows={3}
                    placeholder="e.g. Must study adult populations only (exclude studies on children under 18)"
                    className="w-full px-3 py-2 text-[13px] rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] bg-white placeholder:text-[var(--color-paper-400)] focus:outline-none focus:ring-1 focus:ring-[var(--color-paper-400)] resize-none"
                  />
                  <div className="mt-4 flex gap-3">
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={handleExtraSubmit}
                      disabled={!extraInput.trim()}
                    >
                      Begin screening
                      <Bot size={13} strokeWidth={1.75} />
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => startRun()}>
                      Skip and begin
                    </Button>
                  </div>
                </div>
              )}

              {/* ── Running ── */}
              {screening.step === "running" && (
                <div>
                  {screening.error ? (
                    <div>
                      <p className="text-[13px] text-[var(--color-warn-700)] mb-4">{screening.error}</p>
                      <Button variant="ghost" size="sm" onClick={onDismiss}>Dismiss</Button>
                    </div>
                  ) : (
                    <div>
                      <div className="flex items-center gap-2 mb-4 text-[13px] text-[var(--color-paper-600)]">
                        {screening.isPaused ? (
                          <Pause size={14} className="shrink-0 text-[var(--color-warn-500)]" />
                        ) : (
                          <Loader2 size={14} className="animate-spin shrink-0" />
                        )}
                        <span className="truncate">
                          {screening.isPaused
                            ? "Paused"
                            : screening.progress
                            ? `Screening: ${screening.progress.article_title}`
                            : "Starting…"}
                        </span>
                      </div>

                      {/* Progress bar */}
                      <div className="w-full h-2 rounded-full bg-[var(--color-paper-200)] overflow-hidden mb-2">
                        <motion.div
                          className="h-full rounded-full bg-[var(--color-accent-500)]"
                          initial={{ width: 0 }}
                          animate={{
                            width: screening.progress && screening.progress.total > 0
                              ? `${(screening.progress.done / screening.progress.total) * 100}%`
                              : "0%",
                          }}
                          transition={{ ease: "easeOut", duration: 0.3 }}
                        />
                      </div>

                      <div className="flex items-center justify-between text-[11px] text-[var(--color-paper-500)] tabular-nums mb-5">
                        <span>
                          {screening.progress ? `${screening.progress.done} / ${screening.progress.total}` : "—"}
                        </span>
                        {screening.progress && (
                          <span className={cn(
                            "font-medium",
                            screening.progress.decision === "include" && "text-[var(--color-ok-600)]",
                            screening.progress.decision === "exclude" && "text-[var(--color-warn-600)]",
                            screening.progress.decision === "uncertain" && "text-[var(--color-paper-500)]",
                            screening.progress.decision === "error" && "text-[var(--color-error-500)]",
                          )}>
                            {screening.progress.decision === "include" && "Included"}
                            {screening.progress.decision === "exclude" && "Excluded"}
                            {screening.progress.decision === "uncertain" && "Flagged for review"}
                            {screening.progress.decision === "screening" && "Screening…"}
                            {screening.progress.decision === "error" && "Error — skipped"}
                          </span>
                        )}
                      </div>

                      <div className="flex items-center gap-2">
                        <Button
                          variant={screening.isPaused ? "primary" : "ghost"}
                          size="sm"
                          onClick={handlePauseResume}
                        >
                          {screening.isPaused
                            ? <><Play size={12} strokeWidth={2} /> Resume</>
                            : <><Pause size={12} strokeWidth={2} /> Pause</>}
                        </Button>
                        <Button variant="ghost" size="sm" onClick={onSendToBackground}>
                          Send to background
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              )}

              {/* ── Done ── */}
              {screening.step === "done" && screening.result && (
                <div>
                  <p className="text-[14px] font-medium text-[var(--color-ink-800)] mb-4">
                    Screening complete
                  </p>
                  <div className="grid grid-cols-3 gap-3 mb-5">
                    <StatPill label="Included" value={screening.result.included} color="ok" />
                    <StatPill label="Excluded" value={screening.result.excluded} color="warn" />
                    <StatPill label="Needs review" value={screening.result.uncertain} color="neutral" />
                  </div>
                  <p className="text-[12px] text-[var(--color-paper-500)] leading-relaxed mb-4">
                    Articles flagged for review remain in Pending Screening with an AI rationale.
                    All decisions can be reverted from the article card.
                  </p>
                  <Button variant="primary" size="sm" onClick={() => onComplete(screening.result!)}>
                    View results
                  </Button>
                </div>
              )}
            </Card>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}

function StatPill({ label, value, color }: { label: string; value: number; color: "ok" | "warn" | "neutral" }) {
  const cls = {
    ok: "bg-[var(--color-ok-500)]/10 text-[var(--color-ok-600)]",
    warn: "bg-[var(--color-warn-500)]/10 text-[var(--color-warn-600)]",
    neutral: "bg-[var(--color-paper-100)] text-[var(--color-paper-600)]",
  }[color];
  return (
    <div className={cn("rounded-[var(--radius-sm)] px-3 py-2 text-center", cls)}>
      <div className="text-[22px] font-serif tracking-tight">{value}</div>
      <div className="text-[11px] font-medium mt-0.5">{label}</div>
    </div>
  );
}
