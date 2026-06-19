import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  BookMarked,
  CheckCircle2,
  Download,
  FileSpreadsheet,
  FileText,
  FolderOpen,
  GitBranch,
  ListChecks,
  Loader2,
  ScrollText,
} from "lucide-react";
import { api } from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { cn } from "../../lib/utils";

type Job =
  | "docx"
  | "decisions"
  | "csv"
  | "bibtex"
  | "checklist"
  | "svg"
  | null;

export function ExportPanel() {
  const [busy, setBusy] = useState<Job>(null);
  const [result, setResult] = useState<{ kind: Job; files: string[] } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = async (kind: Exclude<Job, null>) => {
    setBusy(kind);
    setError(null);
    try {
      let files: string[] = [];
      if (kind === "docx") files = [await api.exportDocx()];
      else if (kind === "decisions") files = [await api.exportDecisionReport()];
      else if (kind === "csv") files = (await api.exportCsvs()).files;
      else if (kind === "bibtex") files = [await api.exportBibtex()];
      else if (kind === "checklist") files = [await api.exportPrismaChecklist()];
      else if (kind === "svg") files = [await api.exportPrismaSvg()];
      setResult({ kind, files });
    } catch (e: any) {
      setError(e?.toString() || String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="max-w-4xl">
      <div className="mb-6">
        <div className="eyebrow text-[var(--color-paper-500)]">Protocol-faithful artifacts</div>
        <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">
          Export
        </h2>
        <p className="text-[13px] text-[var(--color-paper-600)] mt-1.5 max-w-xl leading-relaxed">
          Regenerate the SPAR-4-SLR + PRISMA 2020 artifacts from the live database. Hand the
          workspace folder to a journal reviewer and they have everything.
        </p>
      </div>

      <div className="mb-4">
        <ExportCard
          icon={<FileText size={20} strokeWidth={1.5} />}
          title="Full report (DOCX)"
          description="The whole manuscript in one Word document — abstract, method, PRISMA flow, results & synthesis, future agenda, limitations and funding, references — plus a Findings-by-dimension table and the PRISMA 2020 checklist as real Word tables."
          path="drafts/full-report.docx"
          busy={busy === "docx"}
          onRun={() => run("docx")}
        />
      </div>

      <div className="grid grid-cols-2 gap-4">
        <ExportCard
          icon={<ScrollText size={20} strokeWidth={1.5} />}
          title="Decision report (DOCX)"
          description="A human-readable audit trail of every decision — configuration, screening (include/exclude), coding, and AI-assisted resolutions — each labelled human vs AI, for your transparency/methods section."
          path="drafts/decision-report.docx"
          busy={busy === "decisions"}
          onRun={() => run("decisions")}
        />
        <ExportCard
          icon={<FileSpreadsheet size={20} strokeWidth={1.5} />}
          title="Article CSVs"
          description="articles_retrieved.csv, articles_included.csv, articles_excluded.csv, included.bib — all four together."
          path="data/"
          busy={busy === "csv"}
          onRun={() => run("csv")}
        />
        <ExportCard
          icon={<BookMarked size={20} strokeWidth={1.5} />}
          title="BibTeX bibliography"
          description="Stable citation keys for every included article, ready for LaTeX or Word import."
          path="data/included.bib"
          busy={busy === "bibtex"}
          onRun={() => run("bibtex")}
        />
        <ExportCard
          icon={<ListChecks size={20} strokeWidth={1.5} />}
          title="PRISMA 2020 checklist"
          description="The full 27-item reporting checklist, pre-filled with everything Reeves knows about your review."
          path="drafts/prisma_checklist.md"
          busy={busy === "checklist"}
          onRun={() => run("checklist")}
        />
        <ExportCard
          icon={<GitBranch size={20} strokeWidth={1.5} />}
          title="PRISMA flow diagram"
          description="The three-lane PRISMA diagram as a self-contained SVG you can drop into a paper."
          path="prisma/prisma_flow.svg"
          busy={busy === "svg"}
          onRun={() => run("svg")}
        />
      </div>

      <AnimatePresence>
        {result && (
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            className="mt-5 px-4 py-3 rounded-[var(--radius-sm)] bg-[var(--color-ok-500)]/10 border border-[var(--color-ok-500)]/30"
          >
            <div className="flex items-center gap-2 text-[var(--color-ok-500)] text-[12px] font-semibold uppercase tracking-wider">
              <CheckCircle2 size={13} strokeWidth={2} />
              Wrote {result.files.length} file{result.files.length === 1 ? "" : "s"}
            </div>
            <ul className="mt-2 space-y-0.5">
              {result.files.map((f) => (
                <li
                  key={f}
                  className="font-mono text-[11px] text-[var(--color-paper-600)] truncate"
                >
                  {f}
                </li>
              ))}
            </ul>
          </motion.div>
        )}
      </AnimatePresence>
      {error && (
        <div className="mt-4 px-4 py-3 rounded-[var(--radius-sm)] bg-[var(--color-accent-50)] border border-[var(--color-accent-300)] text-[12px] text-[var(--color-accent-700)]">
          {error}
        </div>
      )}

      <div className="mt-6 px-4 py-3 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-200)] flex items-start gap-3">
        <FolderOpen
          size={14}
          strokeWidth={1.75}
          className="text-[var(--color-paper-500)] shrink-0 mt-0.5"
        />
        <div className="text-[12px] text-[var(--color-paper-600)] leading-relaxed">
          The draft (<code className="font-mono">drafts/slr_draft.md</code>) is regenerated from
          the Draft tab. Pandoc-based <code className="font-mono">.docx</code>/
          <code className="font-mono">.tex</code>/<code className="font-mono">.pdf</code> exports
          are a future polish.
        </div>
      </div>
    </div>
  );
}

function ExportCard({
  icon,
  title,
  description,
  path,
  busy,
  onRun,
}: {
  icon: React.ReactNode;
  title: string;
  description: string;
  path: string;
  busy: boolean;
  onRun: () => void;
}) {
  return (
    <Card className={cn("p-5 flex flex-col", busy && "border-[var(--color-ink-700)]")}>
      <div className="flex items-start justify-between mb-3">
        <div className="w-10 h-10 rounded-[var(--radius-sm)] bg-[var(--color-paper-200)] text-[var(--color-ink-700)] flex items-center justify-center">
          {icon}
        </div>
        <span className="font-mono text-[10px] text-[var(--color-paper-500)] truncate max-w-[60%] text-right">
          {path}
        </span>
      </div>
      <h3 className="font-semibold text-[14px] text-[var(--color-ink-900)] mb-1.5 tracking-tight">
        {title}
      </h3>
      <p className="text-[12px] text-[var(--color-paper-600)] leading-relaxed mb-4 flex-1">
        {description}
      </p>
      <Button onClick={onRun} disabled={busy} variant="secondary" size="sm">
        {busy ? <Loader2 size={13} className="animate-spin" /> : <Download size={13} strokeWidth={1.75} />}
        Regenerate
      </Button>
    </Card>
  );
}
