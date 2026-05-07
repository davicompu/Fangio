import { useState, useEffect } from "react";
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
  Loader2,
  Pause,
  Play,
  Quote,
  Sparkles,
  Tags,
  X,
} from "lucide-react";
import type { AiScreeningResult, ArrangingProgress, DimensionInfo, LmModel, ProjectRow } from "../../lib/api";
import { api } from "../../lib/api";
import { ProgressionTabStrip, type ProgressionTab } from "../../components/ProgressionTabStrip";
import { useTabProgress, readManualFlags } from "../../lib/useTabProgress";
import { MarkStepComplete } from "../../components/MarkStepComplete";
import { Button } from "../../components/ui/Button";
import { AiScreeningModal } from "../../components/AiScreeningModal";
import { AiCodingModal } from "../../components/AiCodingModal";
import { useAiCoding } from "../../hooks/useAiCoding";
import { OrganizationPanel } from "./OrganizationPanel";
import { CodingPanel } from "./CodingPanel";
import { CriteriaPanel } from "./CriteriaPanel";
import { ScreeningPanel, useAiScreening } from "./ScreeningPanel";
import { PrismaPanel } from "./PrismaPanel";

interface Props {
  project: ProjectRow;
  onLeaveStage: () => void;
  onProjectChanged: () => Promise<void>;
  lmModels: LmModel[];
  defaultChatModel: string;
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

export function ArrangingStage({ project, onLeaveStage, onProjectChanged, lmModels, defaultChatModel }: Props) {
  const [tab, setTab] = useState<Tab>("organization");
  const [refreshTick, setRefreshTick] = useState(0);
  const [progress, setProgress] = useState<ArrangingProgress | null>(null);
  const [progressSettings, setProgressSettings] = useState<Record<string, unknown>>({});
  const [jumpCodeFilter, setJumpCodeFilter] = useState<{ dimension: string; code: string } | null>(null);
  const [lastEditedArticleId, setLastEditedArticleId] = useState<number | null>(null);
  const [domainExpanded, setDomainExpanded] = useState(false);
  const [aiModalOpen, setAiModalOpen] = useState(false);
  const screening = useAiScreening();
  const coding = useAiCoding();
  const [aiCodingModalOpen, setAiCodingModalOpen] = useState(false);
  const [dimensions, setDimensions] = useState<DimensionInfo[]>([]);

  useEffect(() => {
    api.listFrameworkDimensions().then(setDimensions).catch(() => {});
  }, [project.id, refreshTick]);

  useEffect(() => {
    api.arrangingProgress().then(setProgress).catch(() => {});
    api.getSettings().then(setProgressSettings).catch(() => {});
  }, [project.id, refreshTick]);

  const onChanged = async () => {
    await onProjectChanged();
    setRefreshTick((t) => t + 1);
  };

  const TAB_IDS: Tab[] = ["organization", "criteria", "screening", "coding", "prisma"];
  const manualFlags = readManualFlags("arranging", TAB_IDS, progressSettings);
  const status = useTabProgress<Tab>(tab, {
    organization: () => (progress?.codes_count ?? 0) >= 1,
    criteria: () => (progress?.criteria_count ?? 0) >= 1,
    screening: () => progress != null && progress.articles_retrieved > 0 && progress.articles_screened >= progress.articles_retrieved,
    coding: () => progress != null && progress.articles_included > 0 && progress.articles_coded >= progress.articles_included,
    prisma: () => false, // manual only
  }, manualFlags);

  const numberedTabs: ProgressionTab<Tab>[] = tabs.map((t, i) => ({
    id: t.id,
    label: t.label,
    icon: t.icon,
    status: status[t.id],
    number: i + 1,
  }));

  const handleAiComplete = (_result: AiScreeningResult) => {
    setAiModalOpen(false);
    screening.reset();
    onChanged();
  };

  const handleCancelFromBanner = async () => {
    await screening.cancel();
    onChanged();
    screening.reset();
  };

  const handleCodingComplete = () => {
    setAiCodingModalOpen(false);
    coding.reset();
    onChanged();
  };

  const handleCancelCodingFromBanner = async () => {
    await coding.cancel();
    onChanged();
    coding.reset();
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
        <ProgressionTabStrip
          tabs={numberedTabs}
          onSelect={setTab}
          underlineLayoutId="arranging-tab-underline"
        />
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
              {tab === "coding" && <CodingPanel project={project} refreshKey={refreshTick} onChanged={onChanged} initialCodeFilter={jumpCodeFilter} onCodeFilterConsumed={() => setJumpCodeFilter(null)} lastEditedArticleId={lastEditedArticleId} onArticleEdited={setLastEditedArticleId} onOpenAiCodingModal={() => setAiCodingModalOpen(true)} />}
              {tab === "criteria" && <CriteriaPanel project={project} refreshKey={refreshTick} onChanged={onChanged} />}
              {tab === "screening" && <ScreeningPanel project={project} refreshKey={refreshTick} onChanged={onChanged} onOpenAiModal={() => setAiModalOpen(true)} />}
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

              {tab === "prisma" && (
                <div className="mt-3 rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] px-4 py-3">
                  <MarkStepComplete
                    stage="arranging"
                    tab={tab}
                    isComplete={progressSettings[`progress.arranging.${tab}.complete`] === true}
                    onChanged={onChanged}
                  />
                </div>
              )}
            </motion.aside>
          </AnimatePresence>
        </div>
      </div>

      {/* ── AI Coding Modal ── */}
      <AnimatePresence>
        {aiCodingModalOpen && (
          <AiCodingModal
            open={aiCodingModalOpen}
            onClose={() => {
              if (coding.step === "done" || coding.step === "error" || coding.step === "idle") {
                handleCodingComplete();
              } else {
                setAiCodingModalOpen(false);
              }
            }}
            coding={coding}
            availableModels={lmModels}
            availableDimensions={dimensions}
            defaultChatModel={defaultChatModel}
            onSendToBackground={() => setAiCodingModalOpen(false)}
          />
        )}
      </AnimatePresence>

      {/* ── Persistent background screening banner ── */}
      <AnimatePresence>
        {(screening.step === "running" || screening.step === "done") && !aiModalOpen && (
          <motion.div
            key="screening-banner"
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 16 }}
            transition={{ duration: 0.2 }}
            className="fixed bottom-6 left-1/2 -translate-x-1/2 z-30 w-full max-w-lg px-4 pointer-events-none"
          >
            <div className="pointer-events-auto rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-white shadow-[var(--shadow-paper-md)] px-4 py-3">
              {screening.step === "running" && !screening.error && (
                <>
                  <div className="flex items-center gap-3 mb-2">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-1.5 mb-1">
                        {screening.isPaused
                          ? <Pause size={11} strokeWidth={2} className="shrink-0 text-[var(--color-warn-500)]" />
                          : <Loader2 size={11} className="animate-spin shrink-0 text-[var(--color-accent-500)]" />}
                        <span className="text-[11px] font-semibold uppercase tracking-wide text-[var(--color-paper-500)]">
                          {screening.isPaused ? "Screening paused" : "Screening in background"}
                        </span>
                      </div>
                      <p className="text-[12px] text-[var(--color-ink-700)] truncate">
                        {screening.progress
                          ? `${screening.progress.done} / ${screening.progress.total} — ${screening.progress.article_title}`
                          : "Starting…"}
                      </p>
                    </div>
                    <div className="flex items-center gap-1 shrink-0">
                      <button
                        onClick={screening.isPaused ? screening.resume : screening.pause}
                        className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-100)] transition-paper"
                        title={screening.isPaused ? "Resume" : "Pause"}
                      >
                        {screening.isPaused
                          ? <Play size={13} strokeWidth={2} />
                          : <Pause size={13} strokeWidth={2} />}
                      </button>
                      <button
                        onClick={handleCancelFromBanner}
                        className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-100)] transition-paper text-[var(--color-paper-500)] hover:text-[var(--color-error-500)]"
                        title="Cancel screening"
                      >
                        <X size={13} strokeWidth={2} />
                      </button>
                      <Button variant="ghost" size="sm" onClick={() => { setTab("screening"); setAiModalOpen(true); }}>
                        View
                      </Button>
                    </div>
                  </div>
                  {screening.progress && screening.progress.total > 0 && (
                    <div className="w-full h-1 rounded-full bg-[var(--color-paper-200)] overflow-hidden">
                      <motion.div
                        className="h-full rounded-full bg-[var(--color-accent-500)]"
                        animate={{ width: `${(screening.progress.done / screening.progress.total) * 100}%` }}
                        transition={{ ease: "easeOut", duration: 0.3 }}
                      />
                    </div>
                  )}
                </>
              )}

              {screening.step === "running" && screening.error && (
                <div className="flex items-center justify-between gap-3">
                  <p className="text-[12px] text-[var(--color-warn-700)] truncate">{screening.error}</p>
                  <button
                    onClick={screening.reset}
                    className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-100)] transition-paper shrink-0"
                  >
                    <X size={13} strokeWidth={2} />
                  </button>
                </div>
              )}

              {screening.step === "done" && screening.result && (
                <div className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-3 text-[12px]">
                    <span className="text-[var(--color-ink-800)] font-medium">Screening complete</span>
                    <span className="text-[var(--color-ok-600)]">{screening.result.included} included</span>
                    <span className="text-[var(--color-warn-600)]">{screening.result.excluded} excluded</span>
                    <span className="text-[var(--color-paper-500)]">{screening.result.uncertain} review</span>
                  </div>
                  <div className="flex items-center gap-1 shrink-0">
                    <Button variant="primary" size="sm" onClick={() => { setTab("screening"); setAiModalOpen(true); }}>
                      View results
                    </Button>
                    <button
                      onClick={() => { onChanged(); screening.reset(); }}
                      className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-100)] transition-paper text-[var(--color-paper-500)]"
                      title="Dismiss"
                    >
                      <X size={13} strokeWidth={2} />
                    </button>
                  </div>
                </div>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* ── Persistent background coding banner ── */}
      <AnimatePresence>
        {(coding.step === "running" || coding.step === "done") && !aiCodingModalOpen && (
          <motion.div
            key="coding-banner"
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 16 }}
            transition={{ duration: 0.2 }}
            className="fixed bottom-6 left-1/2 -translate-x-1/2 z-30 w-full max-w-lg px-4 pointer-events-none"
            style={{ bottom: (screening.step === "running" || screening.step === "done") && !aiModalOpen ? "80px" : "24px" }}
          >
            <div className="pointer-events-auto rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-white shadow-[var(--shadow-paper-md)] px-4 py-3">
              {coding.step === "running" && !coding.error && (
                <>
                  <div className="flex items-center gap-3 mb-2">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-1.5 mb-1">
                        {coding.isPaused
                          ? <Pause size={11} strokeWidth={2} className="shrink-0 text-[var(--color-warn-500)]" />
                          : <Loader2 size={11} className="animate-spin shrink-0 text-[var(--color-accent-500)]" />
                        }
                        <span className="text-[11px] font-semibold uppercase tracking-wide text-[var(--color-paper-500)]">
                          {coding.isPaused ? "Coding paused" : "Coding in background"}
                        </span>
                        {coding.progress && (
                          <span className="text-[11px] text-[var(--color-paper-400)] ml-1 font-mono tabular-nums">
                            {coding.progress.done}/{coding.progress.total}
                          </span>
                        )}
                      </div>
                      {coding.progress && (
                        <p className="text-[12px] text-[var(--color-ink-700)] truncate font-medium leading-snug">
                          {coding.progress.article_title}
                        </p>
                      )}
                    </div>

                    {/* Controls */}
                    <div className="flex items-center gap-1.5 shrink-0">
                      <button
                        onClick={() => coding.isPaused ? coding.resume() : coding.pause()}
                        className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-100)] text-[var(--color-paper-600)] transition-paper"
                        title={coding.isPaused ? "Resume" : "Pause"}
                      >
                        {coding.isPaused
                          ? <Play size={12} strokeWidth={2} />
                          : <Pause size={12} strokeWidth={2} />
                        }
                      </button>
                      <button
                        onClick={handleCancelCodingFromBanner}
                        className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-100)] text-[var(--color-paper-500)] transition-paper"
                        title="Cancel"
                      >
                        <X size={12} strokeWidth={2} />
                      </button>
                      <button
                        onClick={() => { setTab("coding"); setAiCodingModalOpen(true); }}
                        className="px-2.5 py-1 rounded-[var(--radius-xs)] text-[11px] font-semibold bg-[var(--color-accent-50)] text-[var(--color-accent-700)] hover:bg-[var(--color-accent-100)] border border-[var(--color-accent-200)] transition-paper"
                      >
                        View
                      </button>
                    </div>
                  </div>

                  {/* Progress bar */}
                  {coding.progress && coding.progress.total > 0 && (
                    <div className="h-1 rounded-full bg-[var(--color-paper-200)] overflow-hidden">
                      <motion.div
                        animate={{ width: `${Math.round((coding.progress.done / coding.progress.total) * 100)}%` }}
                        transition={{ duration: 0.3 }}
                        className="h-full rounded-full bg-[var(--color-accent-500)]"
                      />
                    </div>
                  )}
                </>
              )}

              {coding.step === "done" && coding.result && (
                <div className="flex items-center gap-3">
                  <div className="flex-1">
                    <p className="text-[12px] font-semibold text-[var(--color-ink-800)]">AI coding complete</p>
                    <p className="text-[11px] text-[var(--color-paper-500)] mt-0.5">
                      {coding.result.consensus} consensus applied
                      {coding.result.disagreements > 0 && ` · ${coding.result.disagreements} need review`}
                    </p>
                  </div>
                  {coding.result.disagreements > 0 && (
                    <button
                      onClick={() => { setTab("coding"); onChanged(); coding.reset(); }}
                      className="px-2.5 py-1 rounded-[var(--radius-xs)] text-[11px] font-semibold bg-amber-50 text-amber-700 hover:bg-amber-100 border border-amber-200 transition-paper shrink-0"
                    >
                      Review
                    </button>
                  )}
                  <button
                    onClick={() => { onChanged(); coding.reset(); }}
                    className="p-1 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-100)] text-[var(--color-paper-400)] transition-paper"
                  >
                    <X size={12} strokeWidth={2} />
                  </button>
                </div>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      <AiScreeningModal
        open={aiModalOpen}
        project={project}
        criteria={[]}
        screening={screening}
        onComplete={handleAiComplete}
        onDismiss={() => setAiModalOpen(false)}
        onSendToBackground={() => setAiModalOpen(false)}
      />
    </div>
  );
}
