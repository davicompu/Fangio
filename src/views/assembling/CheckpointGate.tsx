import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { ArrowRight, CheckCircle2, Circle, Lock, ShieldCheck } from "lucide-react";
import { api, type AssemblingState } from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { cn } from "../../lib/utils";

interface Props {
  assembling: AssemblingState | null;
  onAdvance: () => void;
}

const SETTING_KEY = "checkpoint.assembling.confirmed";

interface Condition {
  id: string;
  label: string;
  description: string;
  /** auto-checked from data */
  auto?: (s: AssemblingState | null) => boolean;
}

const conditions: Condition[] = [
  {
    id: "sufficient",
    label: "Sufficient body of work",
    description:
      "At least 40 unique records remain after deduplication. Smaller pools rarely sustain a defensible review.",
    auto: (s) => !!s && s.unique_records >= 40,
  },
  {
    id: "no_recent_slr",
    label: "No recent SLR exists",
    description:
      "You've checked that there is no published systematic review on this exact domain in the last five years.",
  },
  {
    id: "no_high_quality_slr",
    label: "No high-quality SLR exists",
    description:
      "Any prior review you found is either older, lower-tier, or has scope mismatches you can articulate.",
  },
  {
    id: "identifiable_gap",
    label: "Identifiable gap remains",
    description:
      "You can name at least one gap your review will address that prior reviews did not — methodological, contextual, theoretical, or constructual.",
  },
];

export function CheckpointGate({ assembling, onAdvance }: Props) {
  const [checked, setChecked] = useState<Set<string>>(new Set());

  // Seed the manual confirmations from persisted settings so they survive reloads.
  useEffect(() => {
    api.getSettings()
      .then((s) => {
        const saved = s[SETTING_KEY];
        if (Array.isArray(saved)) setChecked(new Set(saved.filter((x): x is string => typeof x === "string")));
      })
      .catch(() => {});
  }, []);

  const allChecked =
    conditions.every((c) => {
      if (c.auto) return c.auto(assembling);
      return checked.has(c.id);
    }) && assembling !== null;

  const toggle = (id: string) => {
    setChecked((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      api.setSetting(SETTING_KEY, Array.from(next)).catch(() => {});
      return next;
    });
  };

  return (
    <div className="max-w-3xl">
      <div className="mb-6 flex items-start gap-4">
        <div className="w-11 h-11 rounded-[var(--radius-md)] bg-[var(--color-ink-800)] text-[var(--color-paper-50)] flex items-center justify-center shrink-0">
          <ShieldCheck size={20} strokeWidth={1.75} />
        </div>
        <div>
          <div className="eyebrow text-[var(--color-paper-500)]">Stage 1 → 2 gate</div>
          <h2 className="font-serif text-[28px] tracking-tight text-[var(--color-ink-900)] mt-1">
            The four-condition checkpoint
          </h2>
          <p className="text-[14px] text-[var(--color-paper-600)] mt-2 leading-relaxed">
            SPAR-4-SLR insists on these four conditions before you commit time to coding the
            corpus. They protect you from publishing a redundant review.
          </p>
        </div>
      </div>

      <div className="space-y-3 mb-8">
        {conditions.map((c) => {
          const isAuto = !!c.auto;
          const ok = isAuto ? c.auto!(assembling) : checked.has(c.id);
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
                    <CheckCircle2
                      size={18}
                      strokeWidth={1.75}
                      className="text-[var(--color-ok-500)]"
                    />
                  ) : (
                    <Circle
                      size={18}
                      strokeWidth={1.75}
                      className="text-[var(--color-paper-400)]"
                    />
                  )}
                </div>
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-[14px] text-[var(--color-ink-900)]">
                      {c.label}
                    </span>
                    {isAuto && (
                      <span className="text-[9px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded-sm bg-[var(--color-paper-200)] text-[var(--color-paper-600)]">
                        auto
                      </span>
                    )}
                  </div>
                  <p className="text-[13px] text-[var(--color-paper-600)] mt-1 leading-relaxed">
                    {c.description}
                  </p>
                  {c.id === "sufficient" && assembling && (
                    <p className="text-[12px] mt-2 font-mono tabular-nums">
                      <span
                        className={
                          assembling.unique_records >= 40
                            ? "text-[var(--color-ok-500)]"
                            : "text-[var(--color-warn-500)]"
                        }
                      >
                        {assembling.unique_records}
                      </span>
                      <span className="text-[var(--color-paper-500)]"> / 40 unique records</span>
                    </p>
                  )}
                </div>
              </button>
            </Card>
          );
        })}
      </div>

      <motion.div
        animate={{ opacity: allChecked ? 1 : 0.5 }}
        className="flex items-center gap-3"
      >
        <Button onClick={onAdvance} disabled={!allChecked} variant="primary" size="lg">
          {allChecked ? (
            <>
              Begin Stage 2 — Arranging
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
            Confirm every condition above to unlock
          </span>
        )}
      </motion.div>

      <p className="mt-6 text-[12px] text-[var(--color-paper-500)] leading-relaxed max-w-xl">
        Stage 2 (Arranging) is the next session's build. The gate is wired up and the decisions
        you confirm here will be persisted, so when Stage 2 ships you can step straight in.
      </p>
    </div>
  );
}
