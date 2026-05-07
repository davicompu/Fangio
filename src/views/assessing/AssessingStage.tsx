import { useState, useEffect } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  ArrowLeft,
  Download,
  Library,
  Lightbulb,
  PenLine,
  Search,
  Sparkles,
} from "lucide-react";
import type { ProjectRow, AssessingProgress } from "../../lib/api";
import { api } from "../../lib/api";
import { PdfLibraryPanel } from "./PdfLibraryPanel";
import { AskCorpusPanel } from "./AskCorpusPanel";
import { DraftPanel } from "./DraftPanel";
import { ExportPanel } from "./ExportPanel";
import { ProgressionTabStrip, type ProgressionTab } from "../../components/ProgressionTabStrip";
import { useTabProgress, readManualFlags } from "../../lib/useTabProgress";

interface Props {
  project: ProjectRow;
  onLeaveStage: () => void;
  onOpenRag: () => void;
}

type Tab = "library" | "ask" | "draft" | "export";

const tabs: { id: Tab; label: string; icon: any }[] = [
  { id: "library", label: "PDF library", icon: Library },
  { id: "ask", label: "Ask the corpus", icon: Search },
  { id: "draft", label: "Draft", icon: PenLine },
  { id: "export", label: "Export", icon: Download },
];

const tabInfo: Record<Tab, { headline: string; body: string; tip?: string }> = {
  library: {
    headline: "Your PDF corpus",
    body: "All included articles with attached PDFs appear here. You can upload full-text PDFs, link them to articles, and build the vector index for corpus Q&A.\n\nArticles are auto-included when a PDF is attached — unless you explicitly excluded them during screening.",
    tip: "Index the corpus before using 'Ask the corpus'. The index is built from your PDFs, not just the abstracts.",
  },
  ask: {
    headline: "Ask the corpus",
    body: "Ask questions of your full-text corpus using retrieval-augmented generation. The AI searches your indexed PDFs and synthesises an answer with citations.\n\nThis is useful for finding patterns, checking claims, and drafting specific sections of your manuscript.",
    tip: "Specific questions get better answers than broad ones. 'Which papers use SEM?' beats 'What methods were used?'",
  },
  draft: {
    headline: "Manuscript draft",
    body: "Fangio generates a skeleton manuscript structured around your research questions and framework. Each section pulls from your decision log and coding data.\n\nEdit the draft directly — it's a starting point, not a finished product.",
    tip: "Run the draft after you've completed coding. The more complete your data, the richer the generated content.",
  },
  export: {
    headline: "Export your review",
    body: "Export your findings, decision log, and PRISMA data in formats suitable for submission or archiving.\n\nThe decision log export provides a full audit trail — useful for peer review and replication.",
    tip: "Export a decision log snapshot before submission. Reviewers increasingly ask for it.",
  },
};

export function AssessingStage({ project, onLeaveStage, onOpenRag }: Props) {
  const [tab, setTab] = useState<Tab>("library");
  const [refreshTick, setRefreshTick] = useState(0);

  const [progress, setProgress] = useState<AssessingProgress | null>(null);
  const [progressSettings, setProgressSettings] = useState<Record<string, unknown>>({});

  useEffect(() => {
    api.assessingProgress().then(setProgress).catch(() => {});
    api.getSettings().then(setProgressSettings).catch(() => {});
  }, [project.id, refreshTick]);

  const onChanged = () => setRefreshTick((t) => t + 1);

  const TAB_IDS: Tab[] = ["library", "ask", "draft", "export"];
  const manualFlags = readManualFlags("assessing", TAB_IDS, progressSettings);
  const status = useTabProgress<Tab>(tab, {
    library: () =>
      progress != null && progress.articles_included > 0 &&
      progress.articles_with_pdf / progress.articles_included >= 0.8,
    ask: () => false, // manual only
    draft: () => !!progress?.has_draft,
    export: () => false, // manual only
  }, manualFlags);

  const numberedTabs: ProgressionTab<Tab>[] = tabs.map((t, i) => ({
    id: t.id,
    label: t.label,
    icon: t.icon,
    status: status[t.id],
    number: i + 1,
  }));

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
              <PenLine size={11} strokeWidth={2} />
              Stage 3 · Assessing
            </div>
            <h1 className="font-serif text-[30px] tracking-tight text-[var(--color-ink-900)] mt-1">
              {project.name}
            </h1>
          </div>
        </div>
        <button
          onClick={onOpenRag}
          className="no-drag group inline-flex items-center gap-2 px-4 h-9 rounded-[var(--radius-sm)] text-[12px] font-semibold bg-[var(--color-accent-50)]0 text-white hover:bg-[var(--color-accent-600)] shadow-sm transition-paper"
        >
          <Sparkles size={13} strokeWidth={1.75} className="text-[var(--color-accent-200)] group-hover:text-white" />
          Ask the literature
        </button>
      </header>

      <nav className="px-6 xl:px-12 mb-6">
        <ProgressionTabStrip
          tabs={numberedTabs}
          onSelect={setTab}
          underlineLayoutId="assessing-tab-underline"
        />
      </nav>

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
              {tab === "library" && (
                <PdfLibraryPanel
                  refreshKey={refreshTick}
                  onChanged={onChanged}
                  manualComplete={progressSettings["progress.assessing.library.complete"] === true}
                />
              )}
              {tab === "ask" && (
                <AskCorpusPanel
                  manualComplete={progressSettings["progress.assessing.ask.complete"] === true}
                  onChanged={onChanged}
                />
              )}
              {tab === "draft" && <DraftPanel project={project} />}
              {tab === "export" && (
                <ExportPanel
                  manualComplete={progressSettings["progress.assessing.export.complete"] === true}
                  onChanged={onChanged}
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
