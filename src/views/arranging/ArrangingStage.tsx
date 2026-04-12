import { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  ArrowLeft,
  Binoculars,
  Boxes,
  ChevronDown,
  ChevronUp,
  GitBranch,
  Layers,
  Lightbulb,
  ListFilter,
  Quote,
  Sparkles,
  Tags,
} from "lucide-react";
import type { ProjectRow } from "../../lib/api";
import { cn } from "../../lib/utils";
import { OrganizationPanel } from "./OrganizationPanel";
import { CodingPanel } from "./CodingPanel";
import { CriteriaPanel } from "./CriteriaPanel";
import { ScreeningPanel } from "./ScreeningPanel";
import { PrismaPanel } from "./PrismaPanel";

interface Props {
  project: ProjectRow;
  onLeaveStage: () => void;
  onProjectChanged: () => Promise<void>;
}

type Tab = "organization" | "coding" | "criteria" | "screening" | "prisma";

const tabs: { id: Tab; label: string; icon: any }[] = [
  { id: "organization", label: "Organisation", icon: Layers },
  { id: "criteria", label: "Purification", icon: ListFilter },
  { id: "screening", label: "Screening", icon: Binoculars },
  { id: "coding", label: "Coding", icon: Tags },
  { id: "prisma", label: "PRISMA flow", icon: GitBranch },
];

const tabInfo: Record<Tab, { headline: string; body: string; tip?: string }> = {
  organization: {
    headline: "Building your codebook",
    body: "Each dimension in your framework becomes a coding category. Within each dimension, you add codes — the specific labels you'll assign to included articles.\n\nFor ADO, your dimensions are Antecedents, Decisions, and Outcomes. For TCM, they're Theories, Contexts, and Methods. For Custom, you name them yourself.\n\nStart by reviewing the default codes and deleting any that don't apply to your domain.",
    tip: "A well-defined codebook is the backbone of a reliable review. Spend time here — it pays off in screening.",
  },
  criteria: {
    headline: "Purification",
    body: "Inclusion criteria specify what an article must have to enter your corpus. Exclusion criteria rule out articles that superficially match but shouldn't be included.\n\nWrite criteria that can be applied mechanically — a colleague should reach the same decision given the same article and your criteria list.",
    tip: "Keep criteria concise. Vague criteria lead to inconsistent screening decisions.",
  },
  screening: {
    headline: "Title & abstract screening",
    body: "Work through each article: read the title and abstract, then decide whether it meets your inclusion criteria.\n\nIncluded articles proceed to full-text assessment and coding. Excluded articles are logged with a reason — essential for PRISMA reporting.\n\nIf you're unsure, mark it 'Include' and resolve at full-text stage.",
    tip: "Aim for decisions in 30–60 seconds per article. If you're deliberating longer, the criteria may need sharpening.",
  },
  coding: {
    headline: "Coding included articles",
    body: "For each included article, assign codes from your framework dimensions. An article can receive multiple codes within a dimension.\n\nCodes you assign here populate the PRISMA flow and feed directly into the Organisation panel's frequency counts — the basis of your manuscript's findings table.",
    tip: "Code conservatively. Only assign a code if the article explicitly addresses that concept.",
  },
  prisma: {
    headline: "PRISMA 2020 flow",
    body: "PRISMA (Preferred Reporting Items for Systematic Reviews and Meta-Analyses) requires you to report how many records were identified, screened, assessed, and included at each stage.\n\nThis panel auto-generates the flow from your actual decisions. Any gaps in the counts indicate steps that still need completing.",
    tip: "The PRISMA flow is a journal requirement for most systematic reviews. Keep it up to date as you screen.",
  },
};

export function ArrangingStage({ project, onLeaveStage, onProjectChanged }: Props) {
  const [tab, setTab] = useState<Tab>("organization");
  const [refreshTick, setRefreshTick] = useState(0);
  const [jumpCodeFilter, setJumpCodeFilter] = useState<{ dimension: string; code: string } | null>(null);
  const [lastEditedArticleId, setLastEditedArticleId] = useState<number | null>(null);
  const [domainExpanded, setDomainExpanded] = useState(false);

  const onChanged = async () => {
    await onProjectChanged();
    setRefreshTick((t) => t + 1);
  };

  const handleJumpToCode = (dimension: string, code: string) => {
    setJumpCodeFilter({ dimension, code });
    setTab("coding");
  };

  return (
    <div className="min-h-screen flex flex-col">
      <header className="drag-region px-6 xl:px-12 pt-10 pb-6 flex items-start justify-between">
        <div className="flex items-start gap-4">
          <button
            onClick={onLeaveStage}
            className="no-drag mt-2 p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
            aria-label="Back"
          >
            <ArrowLeft size={16} strokeWidth={1.75} />
          </button>
          <div>
            <div className="eyebrow flex items-center gap-2">
              <Boxes size={11} strokeWidth={2} />
              Stage 2 · Arranging
            </div>
            <h1 className="font-serif text-[30px] tracking-tight text-[var(--color-ink-900)] mt-1">
              {project.name}
            </h1>
          </div>
        </div>
      </header>

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
                  "no-drag relative px-5 pt-3 pb-3.5 text-[13px] font-medium transition-paper flex items-center gap-2",
                  active
                    ? "text-[var(--color-ink-900)]"
                    : "text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)]",
                )}
              >
                <Icon size={14} strokeWidth={1.75} />
                {t.label}
                {active && (
                  <motion.div
                    layoutId="arranging-tab-underline"
                    className="absolute left-0 right-0 -bottom-px h-0.5 bg-[var(--color-ink-700)]"
                    transition={{ type: "spring", stiffness: 500, damping: 35 }}
                  />
                )}
              </button>
            );
          })}
        </div>
      </nav>

      <div className="flex-1 grid grid-cols-[1fr_240px] xl:grid-cols-[1fr_300px] pb-32 min-w-0">
        {/* Main content column */}
        <div className="min-w-0">
          <AnimatePresence mode="wait">
            <motion.div
              key={tab}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.25, ease: [0.2, 0.7, 0.2, 1] }}
            >
              {tab === "organization" && <OrganizationPanel project={project} refreshKey={refreshTick} onChanged={onChanged} onJumpToCode={handleJumpToCode} />}
              {tab === "coding" && <CodingPanel project={project} refreshKey={refreshTick} onChanged={onChanged} initialCodeFilter={jumpCodeFilter} onCodeFilterConsumed={() => setJumpCodeFilter(null)} lastEditedArticleId={lastEditedArticleId} onArticleEdited={setLastEditedArticleId} />}
              {tab === "criteria" && <CriteriaPanel project={project} refreshKey={refreshTick} onChanged={onChanged} />}
              {tab === "screening" && <ScreeningPanel project={project} refreshKey={refreshTick} onChanged={onChanged} />}
              {tab === "prisma" && <PrismaPanel refreshKey={refreshTick} />}
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

              {/* Domain reminder (if available) — moved here from top banner */}
              {project.domain && (
                <div className="mt-4 rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] overflow-hidden">
                  <button
                    type="button"
                    onClick={() => setDomainExpanded((v) => !v)}
                    className="no-drag w-full flex items-center justify-between px-4 py-3 bg-[var(--color-paper-100)] border-b border-[var(--color-paper-200)] hover:bg-[var(--color-paper-150)] transition-paper text-left"
                  >
                    <div className="flex items-center gap-2">
                      <Quote size={12} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
                      <span className="text-[12px] font-medium text-[var(--color-ink-700)]">Domain scope</span>
                    </div>
                    {domainExpanded
                      ? <ChevronUp size={12} strokeWidth={1.75} className="text-[var(--color-paper-400)]" />
                      : <ChevronDown size={12} strokeWidth={1.75} className="text-[var(--color-paper-400)]" />
                    }
                  </button>
                  <AnimatePresence initial={false}>
                    {domainExpanded && (
                      <motion.div
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: "auto", opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        transition={{ duration: 0.2 }}
                        className="overflow-hidden"
                      >
                        <div className="px-4 py-3">
                          <p className="text-[13px] text-[var(--color-ink-800)] leading-[1.6]">
                            {project.domain}
                          </p>
                          <p className="mt-2 text-[11px] text-[var(--color-paper-400)]">
                            Edit from the project hub (back → pencil icon).
                          </p>
                        </div>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </div>
              )}
            </motion.aside>
          </AnimatePresence>
        </div>
      </div>
    </div>
  );
}
