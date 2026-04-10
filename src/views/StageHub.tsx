import { motion } from "framer-motion";
import {
  ArrowUpRight,
  Boxes,
  CheckCircle2,
  Compass,
  Lock,
  PenLine,
  Quote,
  Search,
  Settings as SettingsIcon,
} from "lucide-react";
import type { ProjectRow } from "../lib/api";
import { Card } from "../components/ui/Card";
import { cn } from "../lib/utils";

interface StageHubProps {
  project: ProjectRow;
  onOpenSettings: () => void;
  onProjectChanged: () => void;
  onEnterStage: (stage: "assembling" | "arranging" | "assessing") => void;
}

const stages = [
  {
    id: "assembling",
    eyebrow: "Stage 1",
    name: "Assembling",
    description:
      "Identification and acquisition: who, what, where, when, and how to search.",
    icon: Search,
    substages: ["Identification", "Acquisition", "Importing", "Deduplication"],
  },
  {
    id: "arranging",
    eyebrow: "Stage 2",
    name: "Arranging",
    description: "Organisation by your framework, then purification against criteria.",
    icon: Boxes,
    substages: ["Organisation", "Purification"],
  },
  {
    id: "assessing",
    eyebrow: "Stage 3",
    name: "Assessing",
    description: "Evaluation, agenda, and a draft manuscript ready for your edits.",
    icon: PenLine,
    substages: ["Evaluation", "Reporting", "Draft"],
  },
];

export function StageHub({ project, onOpenSettings, onProjectChanged, onEnterStage }: StageHubProps) {
  const currentStage = project.current_stage;

  return (
    <div className="min-h-screen flex flex-col">
      <div className="drag-region h-10" />

      {/* Top bar — light, editorial */}
      <div className="px-12 pt-2 pb-8 flex items-start justify-between">
        <div>
          <span className="eyebrow">In progress</span>
          <h1 className="font-serif text-[32px] tracking-tight text-[var(--color-ink-900)] mt-1">
            {project.name}
          </h1>
          <p className="text-[13px] text-[var(--color-paper-600)] mt-1.5 max-w-2xl">
            {project.framework} framework · {project.slr_type} review
          </p>
        </div>
        <div className="no-drag flex items-center gap-2">
          <button
            onClick={onOpenSettings}
            className="inline-flex items-center gap-1.5 px-3 h-8 rounded-[var(--radius-sm)] text-[12px] text-[var(--color-paper-600)] hover:bg-[var(--color-paper-100)] hover:text-[var(--color-ink-800)] transition-paper"
          >
            <SettingsIcon size={13} strokeWidth={1.75} />
            Settings
          </button>
        </div>
      </div>

      {/* Research Question summary card — anchors the whole project */}
      <div className="px-12 mb-10">
        <Card className="p-7">
          <div className="flex items-center gap-2 mb-4 text-[var(--color-paper-500)]">
            <Compass size={13} strokeWidth={1.75} />
            <span className="eyebrow">Research questions</span>
          </div>
          <div className="space-y-4">
            <RqLine label="What" body={project.rq_what || ""} />
            <RqLine label="How" body={project.rq_how || ""} />
            <RqLine label="Where" body={project.rq_where || ""} />
          </div>
          {project.domain && (
            <>
              <div className="my-5 hairline" />
              <div className="flex items-start gap-3">
                <Quote
                  size={13}
                  strokeWidth={1.75}
                  className="text-[var(--color-paper-500)] mt-1 shrink-0"
                />
                <p className="prose-editorial text-[14px] text-[var(--color-paper-700)] italic leading-relaxed">
                  {project.domain}
                </p>
              </div>
            </>
          )}
        </Card>
      </div>

      {/* Stage cards */}
      <div className="px-12 pb-32">
        <div className="flex items-center gap-3 mb-5">
          <span className="eyebrow">SPAR-4-SLR pipeline</span>
          <div className="flex-1 hairline" />
        </div>
        <div className="grid grid-cols-3 gap-5">
          {stages.map((stage, i) => {
            const isCurrent = stage.id === currentStage;
            const isPast =
              stages.findIndex((s) => s.id === currentStage) > i;
            const Icon = stage.icon;
            return (
              <motion.div
                key={stage.id}
                initial={{ opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.5, delay: i * 0.08 }}
              >
                <Card
                  hoverable
                  className={cn(
                    "p-6 h-full",
                    isCurrent && "border-[var(--color-ink-700)] shadow-[var(--shadow-paper-md)]",
                  )}
                >
                  <div className="flex items-start gap-4">
                    <div
                      className={cn(
                        "shrink-0 w-11 h-11 rounded-[var(--radius-md)] flex items-center justify-center",
                        isCurrent
                          ? "bg-[var(--color-ink-800)] text-[var(--color-paper-50)]"
                          : isPast
                          ? "bg-[var(--color-ok-500)]/15 text-[var(--color-ok-500)]"
                          : "bg-[var(--color-paper-200)] text-[var(--color-paper-500)]",
                      )}
                    >
                      {isPast ? (
                        <CheckCircle2 size={20} strokeWidth={1.75} />
                      ) : !isCurrent && !isPast ? (
                        <Lock size={16} strokeWidth={1.75} />
                      ) : (
                        <Icon size={20} strokeWidth={1.75} />
                      )}
                    </div>
                    <div className="flex-1">
                      <div className="eyebrow text-[var(--color-paper-500)]">{stage.eyebrow}</div>
                      <h3 className="font-serif text-[22px] tracking-tight text-[var(--color-ink-900)] mt-0.5">
                        {stage.name}
                      </h3>
                    </div>
                  </div>

                  <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed mt-4 mb-5">
                    {stage.description}
                  </p>

                  <ul className="space-y-1.5">
                    {stage.substages.map((sub) => (
                      <li
                        key={sub}
                        className="text-[12px] flex items-center gap-2 text-[var(--color-paper-600)]"
                      >
                        <span className="w-1 h-1 rounded-full bg-[var(--color-paper-400)]" />
                        {sub}
                      </li>
                    ))}
                  </ul>

                  <div className="mt-6 pt-5 border-t border-[var(--color-paper-200)]">
                    <button
                      type="button"
                      onClick={() => onEnterStage(stage.id as any)}
                      className="inline-flex items-center gap-1.5 text-[12px] font-medium text-[var(--color-ink-700)] hover:text-[var(--color-ink-900)] transition-paper"
                    >
                      Open
                      <ArrowUpRight size={13} strokeWidth={2} />
                    </button>
                  </div>
                </Card>
              </motion.div>
            );
          })}
        </div>

      </div>
    </div>
  );
}

function RqLine({ label, body }: { label: string; body: string }) {
  return (
    <div className="flex gap-4">
      <div className="w-12 shrink-0 text-right">
        <span className="font-serif italic text-[15px] text-[var(--color-paper-500)]">{label}</span>
      </div>
      <p className="prose-editorial text-[15px] text-[var(--color-ink-800)] leading-[1.6] flex-1">
        {body || <span className="text-[var(--color-paper-400)]">Not yet recorded</span>}
      </p>
    </div>
  );
}
