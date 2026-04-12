import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  ArrowLeft,
  BookOpen,
  Compass,
  GitMerge,
  Inbox,
  Library,
  Lightbulb,
  Search,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { api, type AssemblingState, type ProjectRow } from "../../lib/api";
import type { AppSettings } from "../../lib/useAppState";
import { cn } from "../../lib/utils";
import { ImportPanel } from "./ImportPanel";
import { ArticleLibrary } from "./ArticleLibrary";
import { DedupReview } from "./DedupReview";
import { AcquisitionForm } from "./AcquisitionForm";
import { CheckpointGate } from "./CheckpointGate";
import { JournalScopePanel } from "./JournalScopePanel";

interface AssemblingStageProps {
  project: ProjectRow;
  settings: AppSettings;
  onLeaveStage: () => void;
  onAdvanceStage: (nextStage: "arranging") => Promise<void>;
  onOpenRag: () => void;
  onOpenSettings: () => void;
}

type Tab = "acquisition" | "journals" | "import" | "library" | "dedup" | "checkpoint";

const tabInfo: Record<Tab, { headline: string; body: string; tip?: string }> = {
  acquisition: {
    headline: "Defining your search",
    body: "Record which databases you'll search, the date range, and the boolean query string you'll run. These decisions are logged and appear in your manuscript's method section.\n\nA good boolean query balances recall (catching everything relevant) with precision (not drowning in noise). Build it from your research question keywords.",
    tip: "Run your query in each database yourself — Fangio stores the results, but you control the search.",
  },
  import: {
    headline: "Importing search results",
    body: "Export results from each database in RIS, BibTeX, Web of Science, or Scopus CSV format. Fangio parses and deduplicates them automatically.\n\nEach import creates a batch you can trace back — essential for PRISMA reporting.",
    tip: "Import from every database separately to preserve provenance. Merged exports lose source information.",
  },
  journals: {
    headline: "Journal scope & quality",
    body: "Use journal quality lists (ABS, ABDC, Scopus) to scope your search and set a quality threshold. Articles below the threshold can be filtered out before screening.\n\nThis step is optional but recommended for domain reviews targeting high-quality outlets.",
    tip: "Agree on a journal quality threshold before screening — changing it mid-process affects reproducibility.",
  },
  library: {
    headline: "Retrieved corpus",
    body: "All records imported across every batch are stored here. You can inspect, attach PDFs, and track which articles have been screened.\n\nRecords with attached PDFs are automatically marked as included unless you explicitly exclude them during screening.",
    tip: "Attaching a PDF to an article auto-includes it — save time by uploading PDFs for your target articles early.",
  },
  dedup: {
    headline: "Deduplication",
    body: "The same article often appears across multiple databases. Fangio clusters likely duplicates based on title and DOI similarity.\n\nReview each cluster and confirm which records are true duplicates. Merged records keep the richest metadata from the set.",
    tip: "When in doubt, merge. It's better to deduplicate conservatively and focus screening on unique records.",
  },
  checkpoint: {
    headline: "Stage 1 → 2 gate",
    body: "SPAR-4-SLR requires four conditions before you commit time to coding. Confirming them now protects you from publishing a redundant or underpowered review.\n\nAll four must be satisfied before Stage 2 (Arranging) unlocks.",
    tip: "The 40-record threshold is a guideline. Very focused topics can be defensible with fewer, but be prepared to justify it.",
  },
};

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
    description: "Drop RIS, BibTeX, WoS or Scopus exports — Reeves parses and stores them.",
  },
  {
    id: "journals",
    label: "Journals",
    eyebrow: "Scope & quality",
    icon: BookOpen,
    description: "Curated journal list with ABS and ABDC ratings to scope your search.",
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

export function AssemblingStage({ project, settings, onLeaveStage, onAdvanceStage, onOpenRag, onOpenSettings }: AssemblingStageProps) {
  const [tab, setTab] = useState<Tab>("acquisition");
  const [assembling, setAssembling] = useState<AssemblingState | null>(null);
  const [refreshTick, setRefreshTick] = useState(0);

  useEffect(() => {
    api.assemblingState().then(setAssembling).catch(() => {});
  }, [refreshTick]);

  const onChanged = () => setRefreshTick((t) => t + 1);

  return (
    <div className="min-h-screen flex flex-col">
      {/* Header doubles as drag region so the entire top bar is draggable */}
      <header className="drag-region px-6 xl:px-12 pt-10 pb-6 flex items-start justify-between">
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
        <div className="no-drag flex items-center gap-3">
          <button
            onClick={onOpenRag}
            className="group inline-flex items-center gap-2 px-4 h-9 rounded-[var(--radius-sm)] text-[12px] font-semibold bg-[var(--color-accent-50)]0 text-white hover:bg-[var(--color-accent-600)] shadow-sm transition-paper"
          >
            <Sparkles size={13} strokeWidth={1.75} className="text-[var(--color-accent-200)] group-hover:text-white" />
            Ask the literature
          </button>
          <StatsStrip assembling={assembling} />
        </div>
      </header>

      {/* Tab strip — feels like a notebook tab divider */}
      <nav className="px-6 xl:px-12 mb-6">
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

      {/* Tab content — two columns */}
      <div className="flex-1 grid grid-cols-[1fr_240px] xl:grid-cols-[1fr_300px] pb-32 min-w-0">
        {/* Main column */}
        <div className="min-w-0">
          <AnimatePresence mode="wait">
            <motion.div
              key={tab}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.25, ease: [0.2, 0.7, 0.2, 1] }}
              className="pl-6 pr-4 xl:pl-12 xl:pr-8"
            >
              {tab === "acquisition" && (
                <AcquisitionForm
                  project={project}
                  settings={settings}
                  onSaved={onChanged}
                  onOpenSettings={onOpenSettings}
                />
              )}
              {tab === "journals" && <JournalScopePanel />}
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
                  onAdvance={() => onAdvanceStage("arranging")}
                />
              )}
            </motion.div>
          </AnimatePresence>
        </div>

        {/* Right info column */}
        <div className="pr-6 pl-3 xl:pr-12 xl:pl-4 pt-2">
          <AnimatePresence mode="wait">
            <motion.aside
              key={tab + "-info"}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.3, delay: 0.05 }}
              className="sticky top-8"
            >
              <div className="rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] overflow-hidden">
                <div className="flex items-center gap-2 px-4 py-3 bg-[var(--color-paper-100)] border-b border-[var(--color-paper-200)]">
                  <Lightbulb size={13} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                  <span className="eyebrow text-[var(--color-paper-600)]">About this step</span>
                </div>
                <div className="px-4 py-4 space-y-3">
                  <h3 className="font-semibold text-[14px] text-[var(--color-ink-900)] tracking-tight leading-snug">
                    {tabInfo[tab].headline}
                  </h3>
                  <p className="text-[13px] leading-[1.65] whitespace-pre-line text-[var(--color-paper-700)]">
                    {tabInfo[tab].body}
                  </p>
                </div>
                {tabInfo[tab].tip && (
                  <div className="px-4 py-3 bg-[var(--color-accent-50)] border-t border-[var(--color-accent-100)] flex gap-2">
                    <Sparkles size={12} strokeWidth={1.75} className="text-[var(--color-accent-500)] shrink-0 mt-0.5" />
                    <p className="text-[11px] text-[var(--color-accent-700)] leading-relaxed">
                      {tabInfo[tab].tip}
                    </p>
                  </div>
                )}
              </div>
            </motion.aside>
          </AnimatePresence>
        </div>
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
