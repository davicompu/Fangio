import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { motion, AnimatePresence } from "framer-motion";
import {
  CheckCircle2,
  FileText,
  Layers,
  Loader2,
  Plus,
  Sparkles,
  Trash2,
  Upload,
} from "lucide-react";
import { api, type ArticleRow, type PdfRow } from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { Select } from "../../components/ui/Select";
import { cn } from "../../lib/utils";

interface Props {
  refreshKey: number;
  onChanged: () => void;
}

export function PdfLibraryPanel({ refreshKey, onChanged }: Props) {
  const [pdfs, setPdfs] = useState<PdfRow[]>([]);
  const [included, setIncluded] = useState<ArticleRow[]>([]);
  const [busy, setBusy] = useState(false);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = async () => {
    const [p, all] = await Promise.all([api.listPdfs(), api.listArticles()]);
    setPdfs(p);
    setIncluded(all.filter((a) => a.status === "included"));
  };

  useEffect(() => {
    reload();
  }, [refreshKey]);

  const pickPdf = async () => {
    setError(null);
    const sel = await open({
      multiple: false,
      title: "Select a PDF",
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (!sel || Array.isArray(sel)) return;
    setBusy(true);
    try {
      await api.addPdf(sel as string);
      await reload();
      onChanged();
    } catch (e: any) {
      setError(e?.toString() || String(e));
    } finally {
      setBusy(false);
    }
  };

  const link = async (pdfId: number, articleId: number) => {
    setBusyId(pdfId);
    try {
      await api.linkPdf(pdfId, articleId);
      await reload();
      onChanged();
    } finally {
      setBusyId(null);
    }
  };

  const index = async (pdfId: number) => {
    setBusyId(pdfId);
    setError(null);
    try {
      await api.indexPdf(pdfId);
      await reload();
      onChanged();
    } catch (e: any) {
      setError(e?.toString() || String(e));
    } finally {
      setBusyId(null);
    }
  };

  const embed = async (pdfId: number) => {
    setBusyId(pdfId);
    setError(null);
    try {
      await api.embedPdf(pdfId);
      await reload();
      onChanged();
    } catch (e: any) {
      setError(e?.toString() || String(e));
    } finally {
      setBusyId(null);
    }
  };

  const remove = async (pdfId: number) => {
    setBusyId(pdfId);
    try {
      await api.deletePdf(pdfId);
      await reload();
      onChanged();
    } finally {
      setBusyId(null);
    }
  };

  return (
    <div className="max-w-5xl">
      <div className="flex items-end justify-between mb-6">
        <div>
          <div className="eyebrow text-[var(--color-paper-500)]">Project library</div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">
            PDFs and the corpus index
          </h2>
          <p className="text-[13px] text-[var(--color-paper-600)] mt-1.5 max-w-xl leading-relaxed">
            Drop in the full-text PDFs for your included articles. Carta extracts the text
            locally, chunks it, and embeds it via your local model so the Ask the corpus tool can
            cite real passages.
          </p>
        </div>
        <Button onClick={pickPdf} variant="primary" disabled={busy}>
          {busy ? <Loader2 size={14} className="animate-spin" /> : <Plus size={14} strokeWidth={2} />}
          Add PDF
        </Button>
      </div>

      {error && (
        <div className="mb-4 px-4 py-3 rounded-[var(--radius-sm)] bg-[var(--color-accent-50)] border border-[var(--color-accent-300)] text-[13px] text-[var(--color-accent-700)]">
          {error}
        </div>
      )}

      {pdfs.length === 0 ? (
        <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-12 text-center">
          <Upload className="mx-auto mb-3 text-[var(--color-paper-400)]" size={28} strokeWidth={1.5} />
          <p className="text-[14px] text-[var(--color-paper-700)] font-medium mb-1">
            No PDFs yet
          </p>
          <p className="text-[13px] text-[var(--color-paper-500)] max-w-md mx-auto leading-relaxed">
            PDFs are stored inside your project workspace folder under <code className="font-mono">pdfs/</code>. Linking, indexing, and embedding happen locally.
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          <AnimatePresence>
            {pdfs.map((p) => (
              <PdfRowCard
                key={p.id}
                pdf={p}
                included={included}
                busy={busyId === p.id}
                onLink={(aid) => link(p.id, aid)}
                onIndex={() => index(p.id)}
                onEmbed={() => embed(p.id)}
                onRemove={() => remove(p.id)}
              />
            ))}
          </AnimatePresence>
        </div>
      )}
    </div>
  );
}

function PdfRowCard({
  pdf,
  included,
  busy,
  onLink,
  onIndex,
  onEmbed,
  onRemove,
}: {
  pdf: PdfRow;
  included: ArticleRow[];
  busy: boolean;
  onLink: (articleId: number) => void;
  onIndex: () => void;
  onEmbed: () => void;
  onRemove: () => void;
}) {
  const linked = pdf.article_id !== null;
  const indexed = pdf.indexed_at !== null;
  const embedded = pdf.embedded_at !== null;

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.97 }}
    >
      <Card className="p-5">
        <div className="flex items-start gap-4">
          <div className="w-10 h-12 rounded-[var(--radius-xs)] bg-[var(--color-paper-200)] text-[var(--color-paper-600)] flex items-center justify-center shrink-0">
            <FileText size={18} strokeWidth={1.5} />
          </div>
          <div className="flex-1 min-w-0">
            <div className="font-medium text-[14px] text-[var(--color-ink-900)] truncate">
              {pdf.filename}
            </div>
            <div className="flex flex-wrap items-center gap-3 mt-1 text-[11px] text-[var(--color-paper-500)]">
              {pdf.text_length && (
                <span className="font-mono tabular-nums">
                  {pdf.text_length.toLocaleString()} chars
                </span>
              )}
              <StatusPill on={linked} label="linked" />
              <StatusPill on={indexed} label="indexed" />
              <StatusPill on={embedded} label="embedded" highlight />
            </div>
          </div>
          <button
            onClick={onRemove}
            disabled={busy}
            className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] text-[var(--color-paper-500)] hover:text-[var(--color-error-500)] transition-paper"
          >
            <Trash2 size={13} strokeWidth={1.75} />
          </button>
        </div>

        <div className="mt-4 pt-4 border-t border-[var(--color-paper-200)] grid grid-cols-[1fr_auto_auto] gap-3 items-end">
          <Select
            label={linked ? "Linked article" : "Link to an included article"}
            value={pdf.article_id?.toString() ?? ""}
            onChange={(e) => {
              const v = parseInt(e.target.value, 10);
              if (!isNaN(v)) onLink(v);
            }}
          >
            <option value="">— pick an article —</option>
            {included.map((a) => (
              <option key={a.id} value={a.id}>
                #{a.id} · {a.title.length > 70 ? a.title.slice(0, 70) + "…" : a.title}
              </option>
            ))}
          </Select>
          <Button
            variant={indexed ? "ghost" : "secondary"}
            size="md"
            onClick={onIndex}
            disabled={!linked || busy}
          >
            {busy ? <Loader2 size={13} className="animate-spin" /> : <Layers size={13} strokeWidth={1.75} />}
            {indexed ? "Re-index" : "Index"}
          </Button>
          <Button
            variant={embedded ? "ghost" : "primary"}
            size="md"
            onClick={onEmbed}
            disabled={!indexed || busy}
            title={!indexed ? "Index first" : "Compute embeddings via local model"}
          >
            {busy ? <Loader2 size={13} className="animate-spin" /> : <Sparkles size={13} strokeWidth={1.75} />}
            {embedded ? "Re-embed" : "Embed"}
          </Button>
        </div>
      </Card>
    </motion.div>
  );
}

function StatusPill({
  on,
  label,
  highlight,
}: {
  on: boolean;
  label: string;
  highlight?: boolean;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded-sm",
        on
          ? highlight
            ? "bg-[var(--color-accent-100)] text-[var(--color-accent-700)]"
            : "bg-[var(--color-ok-500)]/15 text-[var(--color-ok-500)]"
          : "bg-[var(--color-paper-200)] text-[var(--color-paper-500)]",
      )}
    >
      {on && <CheckCircle2 size={9} strokeWidth={2.25} />}
      {label}
    </span>
  );
}
