import { useState } from "react";
import { motion } from "framer-motion";
import { ArrowRight, CheckCircle2, Circle, Loader2, Lock, ShieldCheck } from "lucide-react";
import type { ArrangingProgress } from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { cn } from "../../lib/utils";

interface Props {
  progress: ArrangingProgress | null;
  advancing: boolean;
  onAdvance: () => void;
}

interface Condition {
  id: string;
  label: string;
  description: string;
  /** When present the condition is satisfied automatically from the data. */
  auto?: (p: ArrangingProgress | null) => boolean;
}

const conditions: Condition[] = [
  {
    id: "codebook",
    label: "Codebook defined",
    description: "Your framework dimensions hold at least one code to assign during coding.",
    auto: (p) => !!p && p.codes_count >= 1,
  },
  {
    id: "criteria",
    label: "Screening criteria set",
    description: "Inclusion / exclusion criteria are written so screening decisions are reproducible.",
    auto: (p) => !!p && p.criteria_count >= 1,
  },
  {
    id: "screened",
    label: "Title & abstract screening complete",
    description: "Every retrieved record has a screening decision — none left pending.",
    auto: (p) => !!p && p.articles_retrieved > 0 && p.articles_screened >= p.articles_retrieved,
  },
  {
    id: "coded",
    label: "Included articles coded",
    description: "Each article that passed both screening gates has been coded against the framework.",
    auto: (p) => !!p && p.articles_included > 0 && p.articles_coded >= p.articles_included,
  },
  {
    id: "prisma",
    label: "PRISMA flow reviewed",
    description: "You've checked the PRISMA flow tab and the counts at each stage add up.",
  },
  {
    id: "ready",
    label: "Ready for synthesis",
    description: "The corpus is settled — you're ready to evaluate findings and report in Stage 3.",
  },
];

export function ArrangingCheckpoint({ progress, advancing, onAdvance }: Props) {
  const [checked, setChecked] = useState<Set<string>>(new Set());

  const isSatisfied = (c: Condition) => (c.auto ? c.auto(progress) : checked.has(c.id));
  const allChecked = progress !== null && conditions.every(isSatisfied);

  const toggle = (id: string) => {
    setChecked((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const stats: { label: string; value: number }[] = [
    { label: "Codes", value: progress?.codes_count ?? 0 },
    { label: "Criteria", value: progress?.criteria_count ?? 0 },
    { label: "Retrieved", value: progress?.articles_retrieved ?? 0 },
    { label: "Screened", value: progress?.articles_screened ?? 0 },
    { label: "Included", value: progress?.articles_included ?? 0 },
    { label: "Coded", value: progress?.articles_coded ?? 0 },
  ];

  return (
    <div className="max-w-3xl">
      <div className="mb-6 flex items-start gap-4">
        <div className="w-11 h-11 rounded-[var(--radius-md)] bg-[var(--color-ink-800)] text-[var(--color-paper-50)] flex items-center justify-center shrink-0">
          <ShieldCheck size={20} strokeWidth={1.75} />
        </div>
        <div>
          <div className="eyebrow text-[var(--color-paper-500)]">Stage 2 → 3 gate</div>
          <h2 className="font-serif text-[28px] tracking-tight text-[var(--color-ink-900)] mt-1">
            Review &amp; finish Stage 2
          </h2>
          <p className="text-[14px] text-[var(--color-paper-600)] mt-2 leading-relaxed">
            A summary of everything you decided while arranging the corpus. Confirm the checklist
            below to close Stage 2 and open Stage 3 (Assessing).
          </p>
        </div>
      </div>

      {/* Decision summary */}
      <div className="grid grid-cols-3 sm:grid-cols-6 gap-2.5 mb-8">
        {stats.map((s) => (
          <div
            key={s.label}
            className="rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] px-3 py-2.5 text-center"
          >
            <div className="text-[20px] font-semibold tabular-nums text-[var(--color-ink-800)] leading-none">
              {s.value}
            </div>
            <div className="mt-1 text-[10px] uppercase tracking-wide text-[var(--color-paper-500)]">
              {s.label}
            </div>
          </div>
        ))}
      </div>

      <div className="space-y-3 mb-8">
        {conditions.map((c) => {
          const isAuto = !!c.auto;
          const ok = isSatisfied(c);
          return (
            <Card
              key={c.id}
              className={cn(
                "p-4",
                ok && "border-[var(--color-ok-500)]/50 bg-[var(--color-ok-500)]/5",
              )}
            >
              <button
                type="button"
                onClick={() => !isAuto && toggle(c.id)}
                disabled={isAuto}
                className="w-full text-left flex items-start gap-3"
              >
                <div className="mt-0.5 shrink-0">
                  {ok ? (
                    <CheckCircle2 size={18} strokeWidth={1.75} className="text-[var(--color-ok-500)]" />
                  ) : (
                    <Circle size={18} strokeWidth={1.75} className="text-[var(--color-paper-400)]" />
                  )}
                </div>
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-[14px] text-[var(--color-ink-900)]">{c.label}</span>
                    {isAuto && (
                      <span className="text-[9px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded-sm bg-[var(--color-paper-200)] text-[var(--color-paper-600)]">
                        auto
                      </span>
                    )}
                  </div>
                  <p className="text-[13px] text-[var(--color-paper-600)] mt-1 leading-relaxed">
                    {c.description}
                  </p>
                </div>
              </button>
            </Card>
          );
        })}
      </div>

      <motion.div animate={{ opacity: allChecked ? 1 : 0.5 }} className="flex items-center gap-3">
        <Button onClick={onAdvance} disabled={!allChecked || advancing} variant="primary" size="lg">
          {advancing ? (
            <>
              <Loader2 size={16} className="animate-spin" />
              Opening Stage 3…
            </>
          ) : allChecked ? (
            <>
              Finish Stage 2 — open Stage 3
              <ArrowRight size={16} strokeWidth={2.25} />
            </>
          ) : (
            <>
              <Lock size={14} strokeWidth={1.75} />
              Gate locked
            </>
          )}
        </Button>
        {!allChecked && (
          <span className="text-[12px] text-[var(--color-paper-500)]">
            Confirm every item above to unlock
          </span>
        )}
      </motion.div>
    </div>
  );
}
