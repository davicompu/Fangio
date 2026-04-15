import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { motion, AnimatePresence } from "framer-motion";
import { Bot, ChevronRight, Loader2, Plus, Sparkles, X } from "lucide-react";
import { api, type AiScreeningProgress, type AiScreeningResult, type ExclusionCriterionRow, type ProjectRow } from "../lib/api";
import { Button } from "./ui/Button";
import { Card } from "./ui/Card";
import { cn } from "../lib/utils";

interface Props {
  open: boolean;
  project: ProjectRow;
  criteria: ExclusionCriterionRow[]; // EC list from DB (number > 1)
  onComplete: (result: AiScreeningResult) => void;
  onDismiss: () => void;
}

type Step = "checklist" | "extra" | "running" | "done";

// Standard PRISMA criteria always shown in the checklist.
// These mirror the seeded exclusion criteria in 0002_arranging.sql (EC2–EC9).
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

export function AiScreeningModal({ open, project, criteria: _criteria, onComplete, onDismiss }: Props) {
  const [step, setStep] = useState<Step>("checklist");

  // Step 1 — checklist
  const [selectedEc, setSelectedEc] = useState<Set<number>>(new Set());

  // Step 2 — extra criteria
  const [wantsExtra, setWantsExtra] = useState<boolean | null>(null);
  const [extraInput, setExtraInput] = useState("");

  // Step 3 — progress
  const [progress, setProgress] = useState<AiScreeningProgress | null>(null);
  const [result, setResult] = useState<AiScreeningResult | null>(null);
  const [runError, setRunError] = useState<string | null>(null);

  const domain = project.domain ?? null;

  useEffect(() => {
    if (!open) return;
    setStep("checklist");
    setSelectedEc(new Set(STANDARD_CRITERIA.map((c) => c.number)));
    setWantsExtra(null);
    setExtraInput("");
    setProgress(null);
    setResult(null);
    setRunError(null);
  }, [open]);

  // Escape key
  useEffect(() => {
    if (!open || step === "running") return;
    const handler = (e: KeyboardEvent) => { if (e.key === "Escape") onDismiss(); };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, step, onDismiss]);

  const toggleEc = (n: number) => {
    setSelectedEc((prev) => {
      const next = new Set(prev);
      if (next.has(n)) next.delete(n);
      else next.add(n);
      return next;
    });
  };

  const handleExtraDecision = (yes: boolean) => {
    setWantsExtra(yes);
    if (!yes) startRun();
    else setStep("extra");
  };

  const handleExtraSubmit = () => {
    startRun(extraInput.trim() || undefined);
  };

  const startRun = async (extraCriterion?: string) => {
    setStep("running");
    setRunError(null);

    const exclusionCriteria = STANDARD_CRITERIA
      .filter((c) => selectedEc.has(c.number))
      .map((c) => `EC${c.number}: ${c.label}`);
    if (extraCriterion) exclusionCriteria.push(extraCriterion);

    let unlisten: (() => void) | null = null;
    try {
      unlisten = await listen<AiScreeningProgress>("screen:progress", (e) => {
        setProgress(e.payload);
      });
      const res = await api.runAiScreening(exclusionCriteria);
      setResult(res);
      setStep("done");
    } catch (e: any) {
      const msg: string = e?.message ?? String(e);
      setRunError(
        msg.includes("chat model")
          ? "Pick a chat model in Settings first."
          : msg.includes("Connection refused") || msg.includes("error sending request")
          ? "Could not reach LM Studio — check Settings."
          : msg,
      );
    } finally {
      unlisten?.();
    }
  };

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
            onClick={step !== "running" ? onDismiss : undefined}
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
                {step !== "running" && (
                  <button
                    onClick={onDismiss}
                    className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
                  >
                    <X size={14} strokeWidth={1.75} />
                  </button>
                )}
              </div>

              {/* Step: checklist */}
              {step === "checklist" && (
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
                      onClick={() => setStep("extra")}
                    >
                      Continue
                      <ChevronRight size={13} strokeWidth={2} />
                    </Button>
                  </div>
                </div>
              )}

              {/* Step: extra criteria */}
              {step === "extra" && wantsExtra === null && (
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

              {step === "extra" && wantsExtra === true && (
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

              {/* Step: running */}
              {step === "running" && (
                <div>
                  {runError ? (
                    <div>
                      <p className="text-[13px] text-[var(--color-warn-700)] mb-4">{runError}</p>
                      <Button variant="ghost" size="sm" onClick={onDismiss}>Dismiss</Button>
                    </div>
                  ) : (
                    <div>
                      <div className="flex items-center gap-2 mb-4 text-[13px] text-[var(--color-paper-600)]">
                        <Loader2 size={14} className="animate-spin shrink-0" />
                        <span className="truncate">
                          {progress
                            ? `Screening: ${progress.article_title}`
                            : "Starting…"}
                        </span>
                      </div>

                      {/* Progress bar */}
                      <div className="w-full h-2 rounded-full bg-[var(--color-paper-200)] overflow-hidden mb-2">
                        <motion.div
                          className="h-full rounded-full bg-[var(--color-accent-500)]"
                          initial={{ width: 0 }}
                          animate={{
                            width: progress && progress.total > 0
                              ? `${(progress.done / progress.total) * 100}%`
                              : "0%",
                          }}
                          transition={{ ease: "easeOut", duration: 0.3 }}
                        />
                      </div>

                      <div className="flex items-center justify-between text-[11px] text-[var(--color-paper-500)] tabular-nums">
                        <span>
                          {progress ? `${progress.done} / ${progress.total}` : "—"}
                        </span>
                        {progress && (
                          <span className={cn(
                            "font-medium",
                            progress.decision === "include" && "text-[var(--color-ok-600)]",
                            progress.decision === "exclude" && "text-[var(--color-warn-600)]",
                            progress.decision === "uncertain" && "text-[var(--color-paper-500)]",
                            progress.decision === "error" && "text-[var(--color-error-500)]",
                          )}>
                            {progress.decision === "include" && "Included"}
                            {progress.decision === "exclude" && "Excluded"}
                            {progress.decision === "uncertain" && "Flagged for review"}
                            {progress.decision === "screening" && "Screening…"}
                            {progress.decision === "error" && "Error — skipped"}
                          </span>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              )}

              {/* Step: done */}
              {step === "done" && result && (
                <div>
                  <p className="text-[14px] font-medium text-[var(--color-ink-800)] mb-4">
                    Screening complete
                  </p>
                  <div className="grid grid-cols-3 gap-3 mb-5">
                    <StatPill label="Included" value={result.included} color="ok" />
                    <StatPill label="Excluded" value={result.excluded} color="warn" />
                    <StatPill label="Needs review" value={result.uncertain} color="neutral" />
                  </div>
                  <p className="text-[12px] text-[var(--color-paper-500)] leading-relaxed mb-4">
                    Articles flagged for review remain in Pending Screening with an AI rationale.
                    All decisions can be reverted from the article card.
                  </p>
                  <Button variant="primary" size="sm" onClick={() => onComplete(result)}>
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
