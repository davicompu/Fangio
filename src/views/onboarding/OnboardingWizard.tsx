import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { ArrowLeft, ArrowRight, Check, Lightbulb, Sparkles } from "lucide-react";
import { api, type ProjectRow } from "../../lib/api";
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

export function OnboardingWizard({ project, onComplete }: OnboardingWizardProps) {
  const [answers, setAnswers] = useState<Answers>(() => initialAnswersFromProject(project));
  const [stepIndex, setStepIndex] = useState(0);
  const [direction, setDirection] = useState<1 | -1>(1);
  const [busy, setBusy] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);

  const step = STEPS[stepIndex];
  const value = answers[step.decisionKey] ?? "";
  const totalSteps = STEPS.length;
  const progress = ((stepIndex + (value ? 1 : 0)) / (totalSteps + 1)) * 100;
  const isLast = stepIndex === totalSteps - 1;

  const setValue = (v: string) => {
    setAnswers((a) => ({ ...a, [step.decisionKey]: v }));
    if (validationError) setValidationError(null);
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
      if (err) {
        setValidationError(err);
        return;
      }
    }
    if (!value.trim()) {
      setValidationError("Please answer to continue.");
      return;
    }
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
      }
    } finally {
      setBusy(false);
    }
  };

  const back = () => {
    if (stepIndex === 0) return;
    setDirection(-1);
    setStepIndex((i) => i - 1);
  };

  // Cmd/Ctrl+Enter to advance
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
        e.preventDefault();
        next();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  return (
    <div className="min-h-screen flex flex-col">
      {/* Drag region + thin progress bar */}
      <div className="drag-region h-10" />
      <div className="px-12 pb-6">
        <ProgressTrail current={stepIndex} total={totalSteps} progress={progress} />
      </div>

      <div className="flex-1 grid grid-cols-[1fr_360px] gap-12 px-12 pb-32">
        {/* Main column */}
        <div className="max-w-2xl">
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

              <StepInput
                step={step}
                value={value}
                onChange={setValue}
                onSubmit={next}
                error={validationError}
              />

              <div className="mt-10 flex items-center gap-3">
                <Button
                  variant="secondary"
                  size="md"
                  onClick={back}
                  disabled={stepIndex === 0 || busy}
                >
                  <ArrowLeft size={15} strokeWidth={1.75} />
                  Back
                </Button>
                <Button variant="primary" size="md" onClick={next} disabled={busy}>
                  {isLast ? (
                    <>
                      Finish onboarding
                      <Check size={15} strokeWidth={2.25} />
                    </>
                  ) : (
                    <>
                      Continue
                      <ArrowRight size={15} strokeWidth={2.25} />
                    </>
                  )}
                </Button>
                <span className="ml-2 text-[11px] text-[var(--color-paper-500)] font-mono">
                  ⌘ ↵
                </span>
              </div>
            </motion.div>
          </AnimatePresence>
        </div>

        {/* Why this step panel */}
        <div className="pt-2">
          <AnimatePresence mode="wait">
            <motion.aside
              key={step.id + "-why"}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.4, delay: 0.1 }}
              className="sticky top-12"
            >
              <div className="flex items-center gap-2 mb-4">
                <Lightbulb size={14} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                <span className="eyebrow">Why this step?</span>
              </div>
              <h3 className="font-semibold text-[15px] text-[var(--color-ink-900)] mb-3 tracking-tight">
                {step.why.headline}
              </h3>
              <div className="prose-editorial text-[14px] leading-[1.7] whitespace-pre-line text-[var(--color-paper-700)]">
                {step.why.body}
              </div>
              {step.why.citation && (
                <div className="mt-4 pt-4 border-t border-[var(--color-paper-200)]">
                  <span className="eyebrow text-[var(--color-paper-500)]">Source</span>
                  <p className="text-[12px] text-[var(--color-paper-600)] mt-1 italic">
                    {step.why.citation}
                  </p>
                </div>
              )}
              <div className="mt-6 px-3 py-2.5 rounded-[var(--radius-sm)] bg-[var(--color-accent-50)] border border-[var(--color-accent-100)] flex gap-2">
                <Sparkles
                  size={13}
                  strokeWidth={1.75}
                  className="text-[var(--color-accent-500)] shrink-0 mt-0.5"
                />
                <p className="text-[12px] text-[var(--color-accent-700)] leading-relaxed">
                  Your answer is saved as a decision in this project's log. You can revisit and
                  revise it any time without losing later work.
                </p>
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
        {error && (
          <p className="text-xs text-[var(--color-error-500)] mt-2">{error}</p>
        )}
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
          if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
            e.preventDefault();
            onSubmit();
          }
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
        if (e.key === "Enter") {
          e.preventDefault();
          onSubmit();
        }
      }}
    />
  );
}

function ProgressTrail({
  current,
  total,
  progress,
}: {
  current: number;
  total: number;
  progress: number;
}) {
  return (
    <div className="flex items-center gap-3">
      <div className="flex items-center gap-1.5">
        {Array.from({ length: total }).map((_, i) => (
          <div
            key={i}
            className={cn(
              "h-1 rounded-full transition-paper",
              i < current
                ? "w-6 bg-[var(--color-ink-700)]"
                : i === current
                ? "w-10 bg-[var(--color-ink-700)]"
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
