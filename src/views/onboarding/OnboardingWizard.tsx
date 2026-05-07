import { useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { ArrowLeft, ArrowRight, Check, Lightbulb, Sparkles, X } from "lucide-react";
import { Markdown } from "../../components/Markdown";
import { api, type ProjectRow, type StepQuestion } from "../../lib/api";
import { Button } from "../../components/ui/Button";
import { TextField, TextArea } from "../../components/ui/TextField";
import { OptionCard } from "../../components/ui/OptionCard";
import { STEPS, type OnboardingStep } from "./steps";
import { cn } from "../../lib/utils";

interface OnboardingWizardProps {
  project: ProjectRow;
  onComplete: (p: ProjectRow) => void;
}

type Answers = Record<string, string>;

type Phase =
  | { kind: "idle" }
  | { kind: "asking" }
  | { kind: "answering"; questions: StepQuestion[]; answers: Record<number, string> }
  | { kind: "rewriting"; questions: StepQuestion[]; answers: Record<number, string> }
  | { kind: "result"; suggestion: string; rationale: string }
  | { kind: "error"; message: string; recoverable: "asking" | "rewriting" };

function initialAnswersFromProject(p: ProjectRow): Answers {
  return {
    "project.name": p.name && p.name !== "Untitled review" ? p.name : "",
    "project.slr_type": p.slr_type ?? "",
    "project.framework": p.framework ?? "",
    "project.domain": p.domain ?? "",
    "project.rq_what": p.rq_what ?? "",
    "project.rq_how": p.rq_how ?? "",
    "project.rq_where": p.rq_where ?? "",
  };
}

// Steps where the suggestion replaces the field value vs. advisory only
const REPLACEABLE_STEPS = new Set(["name", "domain", "rq_what", "rq_how", "rq_where"]);

export function OnboardingWizard({ project, onComplete }: OnboardingWizardProps) {
  const [answers, setAnswers] = useState<Answers>(() => initialAnswersFromProject(project));
  const [stepIndex, setStepIndex] = useState(0);
  const [direction, setDirection] = useState<1 | -1>(1);
  const [busy, setBusy] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);

  // AI assistant phase machine
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const previousValue = useRef<string | null>(null);

  const step = STEPS[stepIndex];
  const value = answers[step.decisionKey] ?? "";
  const totalSteps = STEPS.length;
  const progress = ((stepIndex + (value ? 1 : 0)) / (totalSteps + 1)) * 100;
  const isLast = stepIndex === totalSteps - 1;

  const setValue = (v: string) => {
    setAnswers((a) => ({ ...a, [step.decisionKey]: v }));
    if (validationError) setValidationError(null);
  };

  const clearAssistant = () => {
    setPhase({ kind: "idle" });
    previousValue.current = null;
  };

  const persistCurrent = async () => {
    await api.saveDecision({
      stage: "onboarding",
      sub_stage: null,
      decision_key: step.decisionKey,
      value_json: JSON.stringify(value),
      rationale: null,
    });
  };

  const next = async () => {
    if (step.validate) {
      const err = step.validate(value);
      if (err) { setValidationError(err); return; }
    }
    if (!value.trim()) { setValidationError("Please answer to continue."); return; }
    setBusy(true);
    try {
      await persistCurrent();
      if (isLast) {
        await api.saveDecision({
          stage: "onboarding",
          sub_stage: null,
          decision_key: "project.onboarding_complete",
          value_json: "true",
          rationale: null,
        });
        const updated = await api.currentProject();
        if (updated) onComplete(updated);
      } else {
        setDirection(1);
        setStepIndex((i) => i + 1);
        clearAssistant();
      }
    } finally {
      setBusy(false);
    }
  };

  const back = () => {
    if (stepIndex === 0) return;
    setDirection(-1);
    setStepIndex((i) => i - 1);
    clearAssistant();
  };

  const errorMessage = (e: unknown): string => {
    const msg: string = (e as { message?: string })?.message ?? String(e);
    if (msg.includes("chat model")) return "Pick a chat model in Settings first.";
    if (msg.includes("Connection refused") || msg.includes("error sending request") || msg.includes("connect error")) {
      return "Could not reach LM Studio — check Settings.";
    }
    return "Unexpected error. Try again.";
  };

  const handleSuggest = async () => {
    setPhase({ kind: "asking" });
    try {
      const result = await api.suggestStep(step.id, value);
      setPhase({ kind: "result", suggestion: result.suggestion, rationale: result.rationale });
    } catch (e) {
      setPhase({ kind: "error", message: errorMessage(e), recoverable: "rewriting" });
    }
  };

  const acceptSuggestion = () => {
    if (phase.kind !== "result") return;
    previousValue.current = value;
    setValue(phase.suggestion);
    setPhase({ kind: "idle" });
  };

  const undoAccept = () => {
    if (previousValue.current === null) return;
    setValue(previousValue.current);
    previousValue.current = null;
  };

  // Cmd/Ctrl+Enter to advance
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "Enter") { e.preventDefault(); next(); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const canSuggest = value.trim().length > 0;
  const isReplaceable = REPLACEABLE_STEPS.has(step.id);

  return (
    <div className="min-h-screen flex flex-col">
      {/* Drag region + progress bar */}
      <div className="drag-region h-10" />
      <div className="px-6 xl:px-12 pb-6">
        <ProgressTrail current={stepIndex} total={totalSteps} progress={progress} />
      </div>

      <div className="flex-1 grid grid-cols-[1fr_280px] xl:grid-cols-[1fr_360px] gap-8 xl:gap-12 px-6 xl:px-12 pb-32">
        {/* Main column */}
        <div>
          <AnimatePresence mode="wait" custom={direction}>
            <motion.div
              key={step.id}
              custom={direction}
              initial={{ opacity: 0, x: direction === 1 ? 24 : -24 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: direction === 1 ? -24 : 24 }}
              transition={{ duration: 0.35, ease: [0.2, 0.7, 0.2, 1] }}
            >
              <div className="eyebrow mb-3">{step.eyebrow}</div>
              <h1 className="font-serif text-[36px] tracking-tight leading-[1.15] text-[var(--color-ink-900)] mb-3">
                {step.title}
              </h1>
              {step.subtitle && (
                <p className="text-[15px] text-[var(--color-paper-600)] leading-relaxed mb-8 max-w-xl">
                  {step.subtitle}
                </p>
              )}

              {/* AI assistant trigger */}
              <div className="flex justify-end mb-2">
                <button
                  type="button"
                  onClick={handleSuggest}
                  disabled={!canSuggest || phase.kind === "asking" || phase.kind === "rewriting"}
                  className={cn(
                    "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-[12px] font-medium transition-paper",
                    canSuggest && phase.kind !== "asking" && phase.kind !== "rewriting"
                      ? "text-[var(--color-accent-600)] border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)]"
                      : "text-[var(--color-paper-400)] border border-[var(--color-paper-200)] cursor-not-allowed",
                  )}
                  title={!canSuggest ? "Write your answer first" : undefined}
                >
                  {phase.kind === "asking" || phase.kind === "rewriting" ? <ThinkingDots small /> : <Sparkles size={12} strokeWidth={2} />}
                  {phase.kind === "asking" || phase.kind === "rewriting" ? "Working…" : "✦ Ask assistant"}
                </button>
              </div>

              {/* AI chat bubble */}
              <AnimatePresence>
                {phase.kind !== "idle" && (
                  <motion.div
                    initial={{ opacity: 0, y: -8 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -8 }}
                    transition={{ duration: 0.25 }}
                    className="mb-4 rounded-[var(--radius-md)] bg-[var(--color-accent-50)] border border-[var(--color-accent-100)] p-4"
                  >
                    <div className="flex items-start gap-3">
                      <div className="shrink-0 w-7 h-7 rounded-full bg-[var(--color-accent-600)] flex items-center justify-center">
                        <Sparkles size={13} strokeWidth={1.75} className="text-white" />
                      </div>
                      <div className="flex-1 min-w-0">
                        {phase.kind === "asking" && (
                          <div className="flex items-center gap-2 py-1">
                            <ThinkingDots />
                            <span className="text-[12px] text-[var(--color-paper-500)]">Working on a rewrite…</span>
                          </div>
                        )}
                        {phase.kind === "rewriting" && (
                          <div className="flex items-center gap-2 py-1">
                            <ThinkingDots />
                            <span className="text-[12px] text-[var(--color-paper-500)]">Working on a rewrite…</span>
                          </div>
                        )}
                        {phase.kind === "error" && (
                          <div className="flex items-start justify-between gap-2">
                            <p className="text-[13px] text-[var(--color-warn-700)]">{phase.message}</p>
                            <button
                              type="button"
                              onClick={() => setPhase({ kind: "idle" })}
                              className="shrink-0 mt-0.5 text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)] transition-paper"
                              aria-label="Close"
                            >
                              <X size={12} strokeWidth={2} />
                            </button>
                          </div>
                        )}
                        {phase.kind === "result" && (
                          <div className="space-y-3">
                            <div className="flex items-start justify-between gap-2">
                              <Markdown className="markdown-sm flex-1">{phase.suggestion}</Markdown>
                              <button
                                type="button"
                                onClick={() => setPhase({ kind: "idle" })}
                                className="shrink-0 mt-0.5 text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)] transition-paper"
                                aria-label="Close"
                              >
                                <X size={12} strokeWidth={2} />
                              </button>
                            </div>
                            {phase.rationale && (
                              <p className="text-[11px] text-[var(--color-accent-600)] leading-snug border-t border-[var(--color-accent-100)] pt-2">
                                {phase.rationale}
                              </p>
                            )}
                            {isReplaceable && (
                              <button
                                type="button"
                                onClick={acceptSuggestion}
                                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-xs)] text-[11px] font-medium bg-[var(--color-accent-600)] text-white hover:bg-[var(--color-accent-700)] transition-paper"
                              >
                                Use this
                              </button>
                            )}
                          </div>
                        )}
                      </div>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>

              <StepInput
                step={step}
                value={value}
                onChange={setValue}
                onSubmit={next}
                error={validationError}
              />

              {/* Undo link after accepting a suggestion */}
              {previousValue.current !== null && (
                <button
                  type="button"
                  onClick={undoAccept}
                  className="mt-2 text-[11px] text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] transition-paper"
                >
                  Undo AI suggestion
                </button>
              )}

              <div className="mt-10 flex items-center gap-3">
                <Button variant="secondary" size="md" onClick={back} disabled={stepIndex === 0 || busy}>
                  <ArrowLeft size={15} strokeWidth={1.75} />
                  Back
                </Button>
                <Button variant="primary" size="md" onClick={next} disabled={busy}>
                  {isLast ? (
                    <>Finish onboarding <Check size={15} strokeWidth={2.25} /></>
                  ) : (
                    <>Continue <ArrowRight size={15} strokeWidth={2.25} /></>
                  )}
                </Button>
                <span className="ml-2 text-[11px] text-[var(--color-paper-500)] font-mono">⌘ ↵</span>
              </div>
            </motion.div>
          </AnimatePresence>
        </div>

        {/* Right panel — why this step */}
        <div className="pt-16">
          <AnimatePresence mode="wait">
            <motion.aside
              key={step.id + "-aside"}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.4, delay: 0.1 }}
              className="sticky top-12 space-y-6"
            >
              {/* Why this step — info box */}
              <div className="rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] overflow-hidden">
                {/* Info box header */}
                <div className="flex items-center gap-2 px-4 py-3 bg-[var(--color-paper-100)] border-b border-[var(--color-paper-200)]">
                  <Lightbulb size={13} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                  <span className="eyebrow text-[var(--color-paper-600)]">Why this step?</span>
                </div>
                {/* Info box body */}
                <div className="px-4 py-4 space-y-3">
                  <h3 className="font-semibold text-[14px] text-[var(--color-ink-900)] tracking-tight leading-snug">
                    {step.why.headline}
                  </h3>
                  <div className="prose-editorial text-[13px] leading-[1.65] whitespace-pre-line text-[var(--color-paper-700)]">
                    {step.why.body}
                  </div>
                  {step.why.citation && (
                    <div className="pt-3 border-t border-[var(--color-paper-200)]">
                      <p className="text-[11px] text-[var(--color-paper-500)] italic leading-snug">
                        {step.why.citation}
                      </p>
                    </div>
                  )}
                </div>
                {/* Footer note */}
                <div className="px-4 py-3 bg-[var(--color-accent-50)] border-t border-[var(--color-accent-100)] flex gap-2">
                  <Sparkles size={12} strokeWidth={1.75} className="text-[var(--color-accent-500)] shrink-0 mt-0.5" />
                  <p className="text-[11px] text-[var(--color-accent-700)] leading-relaxed">
                    Answers are saved to your project's decision log. Revisit and revise from the project hub.
                  </p>
                </div>
              </div>
            </motion.aside>
          </AnimatePresence>
        </div>
      </div>
    </div>
  );
}

function StepInput({
  step,
  value,
  onChange,
  onSubmit,
  error,
}: {
  step: OnboardingStep;
  value: string;
  onChange: (v: string) => void;
  onSubmit: () => void;
  error: string | null;
}) {
  if (step.kind === "choice") {
    return (
      <div className="space-y-3">
        {step.options!.map((opt) => (
          <OptionCard
            key={opt.value}
            selected={value === opt.value}
            onSelect={() => onChange(opt.value)}
            title={opt.label}
            description={opt.description}
            recommended={opt.recommended}
            badge={opt.badge}
          />
        ))}
        {error && <p className="text-xs text-[var(--color-error-500)] mt-2">{error}</p>}
      </div>
    );
  }
  if (step.kind === "longText") {
    return (
      <TextArea
        autoFocus
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={step.placeholder}
        rows={6}
        error={error || undefined}
        hint={step.hint}
        onKeyDown={(e) => {
          if ((e.metaKey || e.ctrlKey) && e.key === "Enter") { e.preventDefault(); onSubmit(); }
        }}
      />
    );
  }
  return (
    <TextField
      autoFocus
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={step.placeholder}
      error={error || undefined}
      hint={step.hint}
      onKeyDown={(e) => {
        if (e.key === "Enter") { e.preventDefault(); onSubmit(); }
      }}
    />
  );
}

function ThinkingDots({ small }: { small?: boolean }) {
  const size = small ? "w-1.5 h-1.5" : "w-2 h-2";
  return (
    <div className="flex items-center gap-1">
      {[0, 1, 2].map((i) => (
        <motion.span
          key={i}
          className={`${size} rounded-full bg-[var(--color-paper-400)]`}
          animate={{ y: [0, -4, 0], opacity: [0.4, 1, 0.4] }}
          transition={{ duration: 0.9, repeat: Infinity, delay: i * 0.18, ease: "easeInOut" }}
        />
      ))}
    </div>
  );
}

function ProgressTrail({ current, total, progress }: { current: number; total: number; progress: number }) {
  return (
    <div className="flex items-center gap-3">
      <div className="flex items-center gap-1.5">
        {Array.from({ length: total }).map((_, i) => (
          <div
            key={i}
            className={cn(
              "h-1 rounded-full transition-paper",
              i < current ? "w-6 bg-[var(--color-ink-700)]"
              : i === current ? "w-10 bg-[var(--color-ink-700)]"
              : "w-6 bg-[var(--color-paper-300)]",
            )}
          />
        ))}
      </div>
      <span className="text-[11px] font-mono tabular-nums text-[var(--color-paper-500)] ml-2">
        {Math.round(progress)}%
      </span>
    </div>
  );
}
