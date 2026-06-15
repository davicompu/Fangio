import { useState, useEffect, useRef } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  ArrowLeft,
  BarChart3,
  Compass,
  Download,
  GitBranch,
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
import { FindingsPanel } from "./FindingsPanel";
import { AgendaPanel } from "./AgendaPanel";
import { DraftPanel } from "./DraftPanel";
import { ExportPanel } from "./ExportPanel";
import { PrismaPanel } from "../arranging/PrismaPanel";
import { ProgressionTabStrip, type ProgressionTab } from "../../components/ProgressionTabStrip";
import { useTabProgress, readManualFlags } from "../../lib/useTabProgress";
import { MarkStepComplete } from "../../components/MarkStepComplete";

interface Props {
  project: ProjectRow;
  onLeaveStage: () => void;
  onOpenRag: () => void;
}

type Tab = "library" | "findings" | "ask" | "agenda" | "prisma" | "draft" | "export";

const tabs: { id: Tab; label: string; icon: any }[] = [
  // 3a · Evaluation
  { id: "library", label: "Corpus", icon: Library },
  { id: "findings", label: "Findings", icon: BarChart3 },
  { id: "ask", label: "Synthesis", icon: Search },
  { id: "agenda", label: "Agenda", icon: Compass },
  // 3b · Reporting
  { id: "prisma", label: "PRISMA", icon: GitBranch },
  { id: "draft", label: "Draft", icon: PenLine },
  { id: "export", label: "Export", icon: Download },
];

const tabInfo: Record<Tab, { headline: string; body: string; tip?: string }> = {
  library: {
    headline: "Evaluation · full-text corpus",
    body: "All included articles with attached PDFs appear here. Upload full-text PDFs, link them to articles, and build the vector index for corpus Q&A.\n\nArticles are auto-included when a PDF is attached — unless you explicitly excluded them during screening.",
    tip: "Index the corpus before using Synthesis. The index is built from your PDFs, not just the abstracts.",
  },
  findings: {
    headline: "Evaluation · findings",
    body: "The evidence base for your review: how often each code was applied across the included corpus, organised by framework dimension. These frequencies are the raw material for your results narrative.\n\nRecord the analysis method you used — it's reported in the manuscript's method section.",
    tip: "Uneven or sparse dimensions often signal the gaps you'll turn into your research agenda.",
  },
  ask: {
    headline: "Evaluation · synthesis",
    body: "Ask questions of your full-text corpus using retrieval-augmented generation. The AI searches your indexed PDFs and synthesises an answer with citations.\n\nUse it to find patterns, check claims, and draft specific sections of your manuscript.",
    tip: "Specific questions get better answers. 'Which papers use SEM?' beats 'What methods were used?'",
  },
  agenda: {
    headline: "Evaluation · research agenda",
    body: "The signature output of a SPAR-4-SLR review: a forward-looking agenda of future research directions, grounded in the gaps your coding revealed.\n\nGenerate a starting point from your research questions and dimension findings, then refine it. It feeds the manuscript's agenda section.",
    tip: "Tie each direction to a specific gap or contradiction in the corpus — reviewers reward concrete agendas.",
  },
  prisma: {
    headline: "Reporting · PRISMA 2020",
    body: "The PRISMA flow diagram and 27-item checklist, generated from your actual decisions. PRISMA 2020 (Page et al., 2021) is the reporting standard most journals require for systematic reviews.\n\nGaps in the counts indicate steps that still need completing back in earlier stages.",
    tip: "Export the flow diagram and checklist with your submission — most journals require both.",
  },
  draft: {
    headline: "Reporting · manuscript draft",
    body: "Reeves assembles a skeleton manuscript from your project state — research questions, PRISMA flow, analysis method, findings, your research agenda, limitations, and funding.\n\nSet the reporting details at the top, then generate and edit the draft directly.",
    tip: "Save your reporting details first, then (re)generate so they're woven into the draft.",
  },
  export: {
    headline: "Reporting · export",
    body: "Export your findings, decision log, and PRISMA data in formats suitable for submission or archiving.\n\nThe decision log export provides a full audit trail — useful for peer review and replication.",
    tip: "Export a decision log snapshot before submission. Reviewers increasingly ask for it.",
  },
};

export function AssessingStage({ project, onLeaveStage, onOpenRag }: Props) {
  const [tab, setTab] = useState<Tab>("library");
  const [refreshTick, setRefreshTick] = useState(0);

  const [progress, setProgress] = useState<AssessingProgress | null>(null);
  const [progressSettings, setProgressSettings] = useState<Record<string, unknown>>({});
  const lastTabHydrated = useRef(false);
  const TAB_IDS: Tab[] = ["library", "findings", "ask", "agenda", "prisma", "draft", "export"];

  useEffect(() => {
    api.assessingProgress().then(setProgress).catch(() => {});
    api.getSettings().then((s) => {
      setProgressSettings(s);
      if (!lastTabHydrated.current) {
        const saved = s["last_tab.assessing"];
        if (typeof saved === "string" && (TAB_IDS as readonly string[]).includes(saved)) {
          setTab(saved as Tab);
        }
        lastTabHydrated.current = true;
      }
    }).catch(() => {
      lastTabHydrated.current = true;
    });
  }, [project.id, refreshTick]);

  useEffect(() => {
    if (lastTabHydrated.current) {
      api.setSetting("last_tab.assessing", tab).catch(() => {});
    }
  }, [tab]);

  const onChanged = () => setRefreshTick((t) => t + 1);
  const manualFlags = readManualFlags("assessing", TAB_IDS, progressSettings);
  const status = useTabProgress<Tab>(tab, {
    library: () =>
      progress != null && progress.articles_included > 0 &&
      progress.articles_with_pdf / progress.articles_included >= 0.8,
    findings: () => false, // reference view (analysis method is derived from the framework)
    ask: () => false, // manual only
    agenda: () => !!(project.research_agenda && project.research_agenda.trim()),
    prisma: () => false, // manual only
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
      <header className="drag-region px-6 xl:px-12 pt-14 pb-6 flex items-start justify-between">
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
              {tab === "library" && <PdfLibraryPanel refreshKey={refreshTick} onChanged={onChanged} />}
              {tab === "findings" && <FindingsPanel project={project} />}
              {tab === "ask" && <AskCorpusPanel />}
              {tab === "agenda" && <AgendaPanel project={project} />}
              {tab === "prisma" && <PrismaPanel refreshKey={refreshTick} />}
              {tab === "draft" && <DraftPanel project={project} />}
              {tab === "export" && <ExportPanel />}
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

              {(tab === "library" || tab === "ask" || tab === "prisma" || tab === "export") && (
                <div className="mt-3">
                  <MarkStepComplete
                    stage="assessing"
                    tab={tab}
                    isComplete={progressSettings[`progress.assessing.${tab}.complete`] === true}
                    onChanged={onChanged}
                  />
                </div>
              )}
            </motion.aside>
          </AnimatePresence>
        </div>
      </div>
    </div>
  );
}
