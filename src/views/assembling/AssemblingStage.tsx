import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  ArrowLeft,
  Compass,
  GitMerge,
  Inbox,
  Library,
  Search,
  ShieldCheck,
} from "lucide-react";
import { api, type AssemblingState, type ProjectRow } from "../../lib/api";
import type { AppSettings } from "../../lib/useAppState";
import { cn } from "../../lib/utils";
import { ImportPanel } from "./ImportPanel";
import { ArticleLibrary } from "./ArticleLibrary";
import { DedupReview } from "./DedupReview";
import { AcquisitionForm } from "./AcquisitionForm";
import { CheckpointGate } from "./CheckpointGate";

interface AssemblingStageProps {
  project: ProjectRow;
  settings: AppSettings;
  onLeaveStage: () => void;
  onOpenSettings: () => void;
}

type Tab = "acquisition" | "import" | "library" | "dedup" | "checkpoint";

const tabs: { id: Tab; label: string; eyebrow: string; icon: any; description: string }[] = [
  {
    id: "acquisition",
    label: "Acquisition",
    eyebrow: "Decisions 5–7",
    icon: Compass,
    description: "Databases, search period, and the boolean query you'll run yourself.",
  },
  {
    id: "import",
    label: "Importing",
    eyebrow: "Bring records in",
    icon: Inbox,
    description: "Drop RIS, BibTeX, WoS or Scopus exports — Fangio parses and stores them.",
  },
  {
    id: "library",
    label: "Library",
    eyebrow: "All retrieved",
    icon: Library,
    description: "Every record across every import batch.",
  },
  {
    id: "dedup",
    label: "Deduplication",
    eyebrow: "Cluster review",
    icon: GitMerge,
    description: "Review proposed duplicate clusters and merge them.",
  },
  {
    id: "checkpoint",
    label: "Checkpoint",
    eyebrow: "Stage 1 → 2 gate",
    icon: ShieldCheck,
    description: "Confirm the four conditions before moving to Arranging.",
  },
];

export function AssemblingStage({ project, settings, onLeaveStage, onOpenSettings }: AssemblingStageProps) {
  const [tab, setTab] = useState<Tab>("acquisition");
  const [assembling, setAssembling] = useState<AssemblingState | null>(null);
  const [refreshTick, setRefreshTick] = useState(0);

  useEffect(() => {
    api.assemblingState().then(setAssembling).catch(() => {});
  }, [refreshTick]);

  const onChanged = () => setRefreshTick((t) => t + 1);

  return (
    <div className="min-h-screen flex flex-col">
      <div className="drag-region h-10" />

      {/* Header */}
      <header className="px-12 pt-2 pb-6 flex items-start justify-between">
        <div className="flex items-start gap-4">
          <button
            onClick={onLeaveStage}
            className="no-drag mt-2 p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
            aria-label="Back to project hub"
          >
            <ArrowLeft size={16} strokeWidth={1.75} />
          </button>
          <div>
            <div className="eyebrow flex items-center gap-2">
              <Search size={11} strokeWidth={2} />
              Stage 1 · Assembling
            </div>
            <h1 className="font-serif text-[30px] tracking-tight text-[var(--color-ink-900)] mt-1">
              {project.name}
            </h1>
          </div>
        </div>
        <StatsStrip assembling={assembling} />
      </header>

      {/* Tab strip — feels like a notebook tab divider */}
      <nav className="px-12 mb-6">
        <div className="flex items-end gap-1 border-b border-[var(--color-paper-300)]">
          {tabs.map((t) => {
            const Icon = t.icon;
            const active = tab === t.id;
            return (
              <button
                key={t.id}
                onClick={() => setTab(t.id)}
                className={cn(
                  "no-drag relative px-5 pt-3 pb-3.5 text-[13px] font-medium",
                  "transition-paper flex items-center gap-2",
                  active
                    ? "text-[var(--color-ink-900)]"
                    : "text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)]",
                )}
              >
                <Icon size={14} strokeWidth={1.75} />
                {t.label}
                {active && (
                  <motion.div
                    layoutId="tab-underline"
                    className="absolute left-0 right-0 -bottom-px h-0.5 bg-[var(--color-ink-700)]"
                    transition={{ type: "spring", stiffness: 500, damping: 35 }}
                  />
                )}
              </button>
            );
          })}
        </div>
      </nav>

      {/* Tab content */}
      <div className="flex-1 px-12 pb-32">
        <AnimatePresence mode="wait">
          <motion.div
            key={tab}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={{ duration: 0.25, ease: [0.2, 0.7, 0.2, 1] }}
          >
            {tab === "acquisition" && (
              <AcquisitionForm
                project={project}
                settings={settings}
                onSaved={onChanged}
                onOpenSettings={onOpenSettings}
              />
            )}
            {tab === "import" && <ImportPanel onCommitted={onChanged} />}
            {tab === "library" && (
              <ArticleLibrary refreshKey={refreshTick} onChanged={onChanged} />
            )}
            {tab === "dedup" && (
              <DedupReview refreshKey={refreshTick} onChanged={onChanged} />
            )}
            {tab === "checkpoint" && (
              <CheckpointGate
                assembling={assembling}
                onAdvance={onLeaveStage /* placeholder */}
              />
            )}
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}

function StatsStrip({ assembling }: { assembling: AssemblingState | null }) {
  return (
    <div className="no-drag flex items-stretch gap-0 border border-[var(--color-paper-300)] rounded-[var(--radius-md)] overflow-hidden bg-[var(--color-paper-100)]">
      <Stat label="Imports" value={assembling?.import_batches} />
      <Stat label="Records" value={assembling?.total_records} />
      <Stat label="Unique" value={assembling?.unique_records} highlight />
      <Stat label="Duplicates" value={assembling?.duplicate_records} muted />
    </div>
  );
}

function Stat({
  label,
  value,
  highlight,
  muted,
}: {
  label: string;
  value: number | null | undefined;
  highlight?: boolean;
  muted?: boolean;
}) {
  return (
    <div className="px-4 py-2.5 border-r last:border-r-0 border-[var(--color-paper-300)] min-w-[88px]">
      <div className="eyebrow text-[var(--color-paper-500)]">{label}</div>
      <div
        className={cn(
          "font-serif text-[22px] tabular-nums leading-none mt-1",
          highlight && "text-[var(--color-ink-900)]",
          muted && "text-[var(--color-paper-500)]",
          !highlight && !muted && "text-[var(--color-ink-800)]",
        )}
      >
        {value ?? "—"}
      </div>
    </div>
  );
}
