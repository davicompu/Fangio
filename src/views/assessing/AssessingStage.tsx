import { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  ArrowLeft,
  Download,
  Library,
  PenLine,
  Search,
} from "lucide-react";
import type { ProjectRow } from "../../lib/api";
import { cn } from "../../lib/utils";
import { PdfLibraryPanel } from "./PdfLibraryPanel";
import { AskCorpusPanel } from "./AskCorpusPanel";
import { DraftPanel } from "./DraftPanel";
import { ExportPanel } from "./ExportPanel";

interface Props {
  project: ProjectRow;
  onLeaveStage: () => void;
}

type Tab = "library" | "ask" | "draft" | "export";

const tabs: { id: Tab; label: string; icon: any }[] = [
  { id: "library", label: "PDF library", icon: Library },
  { id: "ask", label: "Ask the corpus", icon: Search },
  { id: "draft", label: "Draft", icon: PenLine },
  { id: "export", label: "Export", icon: Download },
];

export function AssessingStage({ project, onLeaveStage }: Props) {
  const [tab, setTab] = useState<Tab>("library");
  const [refreshTick, setRefreshTick] = useState(0);
  const onChanged = () => setRefreshTick((t) => t + 1);

  return (
    <div className="min-h-screen flex flex-col">
      <header className="drag-region px-12 pt-10 pb-6 flex items-start justify-between">
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
      </header>

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
                    layoutId="assessing-tab-underline"
                    className="absolute left-0 right-0 -bottom-px h-0.5 bg-[var(--color-ink-700)]"
                    transition={{ type: "spring", stiffness: 500, damping: 35 }}
                  />
                )}
              </button>
            );
          })}
        </div>
      </nav>

      <div className="flex-1 px-12 pb-32">
        <AnimatePresence mode="wait">
          <motion.div
            key={tab}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={{ duration: 0.25, ease: [0.2, 0.7, 0.2, 1] }}
          >
            {tab === "library" && <PdfLibraryPanel refreshKey={refreshTick} onChanged={onChanged} />}
            {tab === "ask" && <AskCorpusPanel />}
            {tab === "draft" && <DraftPanel project={project} />}
            {tab === "export" && <ExportPanel />}
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}
