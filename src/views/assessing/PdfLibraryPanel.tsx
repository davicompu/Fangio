import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { motion, AnimatePresence } from "framer-motion";
import {
  CheckCircle2,
  DownloadCloud,
  ExternalLink,
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
  const [codesByArticle, setCodesByArticle] = useState<Map<number, string[]>>(new Map());
  const [busy, setBusy] = useState(false);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [indexingAll, setIndexingAll] = useState(false);
  const [indexAllMsg, setIndexAllMsg] = useState<string | null>(null);
  const [findingPdfs, setFindingPdfs] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Try to fetch open-access PDFs (via Unpaywall) for included articles that
  // have a DOI but no PDF, saving them in just like a manual drop. Reaches the
  // internet (open-access copies only).
  const findPdfs = async () => {
    setFindingPdfs(true);
    setError(null);
    setIndexAllMsg("Searching Unpaywall for open-access PDFs…");
    try {
      const r = await api.findMissingPdfs();
      const parts = [`${r.found} found`];
      if (r.no_oa > 0) parts.push(`${r.no_oa} no open-access copy`);
      if (r.no_doi > 0) parts.push(`${r.no_doi} skipped (no DOI)`);
      setIndexAllMsg(parts.join(" · "));
      if (r.found > 0) {
        await reload();
        onChanged();
      }
    } catch (e: any) {
      setError(e?.toString() || String(e));
      setIndexAllMsg(null);
    } finally {
      setFindingPdfs(false);
    }
  };

  const reload = async () => {
    const [p, all, batch] = await Promise.all([
      api.listPdfs(),
      api.listArticles(),
      api.listCodingBatch(0, 9999),
    ]);
    setPdfs(p);
    setIncluded(all.filter((a) => a.status === "included"));
    const m = new Map<number, string[]>();
    for (const c of batch.codings) {
      const arr = m.get(c.article_id) ?? [];
      if (!arr.includes(c.code)) arr.push(c.code);
      m.set(c.article_id, arr);
    }
    setCodesByArticle(m);
  };

  useEffect(() => {
    reload();
  }, [refreshKey]);

  // Index + embed every linked PDF that isn't fully embedded yet, in one go —
  // so the researcher doesn't have to remember to do each row by hand.
  const indexCorpus = async () => {
    const pending = pdfs.filter((p) => p.article_id !== null && (!p.indexed_at || !p.embedded_at));
    if (pending.length === 0) {
      setIndexAllMsg("Corpus already indexed and embedded.");
      return;
    }
    setIndexingAll(true);
    setError(null);
    let done = 0;
    try {
      for (const p of pending) {
        setIndexAllMsg(`Indexing ${done + 1} of ${pending.length}…`);
        if (!p.indexed_at) await api.indexPdf(p.id);
        await api.embedPdf(p.id);
        done++;
      }
      setIndexAllMsg(`Indexed and embedded ${done} PDF${done !== 1 ? "s" : ""}.`);
      await reload();
      onChanged();
    } catch (e: any) {
      setError(e?.toString() || String(e));
    } finally {
      setIndexingAll(false);
    }
  };

  const openPdf = async (p: PdfRow) => {
    try {
      await api.openPdfPath(p.stored_path);
    } catch (e: any) {
      setError(e?.toString() || String(e));
    }
  };

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
            Drop in the full-text PDFs for your included articles. Reeves extracts the text
            locally, chunks it, and embeds it via your local model so the Ask the corpus tool can
            cite real passages.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button onClick={findPdfs} variant="secondary" disabled={findingPdfs || busy} title="Fetch open-access PDFs from Unpaywall for included articles that have a DOI but no PDF">
            {findingPdfs ? <Loader2 size={14} className="animate-spin" /> : <DownloadCloud size={14} strokeWidth={1.75} />}
            Find PDFs
          </Button>
          <Button onClick={indexCorpus} variant="secondary" disabled={indexingAll || busy}>
            {indexingAll ? <Loader2 size={14} className="animate-spin" /> : <Layers size={14} strokeWidth={1.75} />}
            Index corpus
          </Button>
          <Button onClick={pickPdf} variant="primary" disabled={busy}>
            {busy ? <Loader2 size={14} className="animate-spin" /> : <Plus size={14} strokeWidth={2} />}
            Add PDF
          </Button>
        </div>
      </div>

      {indexAllMsg && (
        <div className="mb-4 px-4 py-2.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-300)] text-[12px] text-[var(--color-paper-600)]">
          {indexAllMsg}
        </div>
      )}

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
                article={p.article_id != null ? included.find((a) => a.id === p.article_id) ?? null : null}
                codes={p.article_id != null ? codesByArticle.get(p.article_id) ?? [] : []}
                included={included}
                busy={busyId === p.id}
                onOpen={() => openPdf(p)}
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
  article,
  codes,
  included,
  busy,
  onOpen,
  onLink,
  onIndex,
  onEmbed,
  onRemove,
}: {
  pdf: PdfRow;
  article: ArticleRow | null;
  codes: string[];
  included: ArticleRow[];
  busy: boolean;
  onOpen: () => void;
  onLink: (articleId: number) => void;
  onIndex: () => void;
  onEmbed: () => void;
  onRemove: () => void;
}) {
  const linked = pdf.article_id !== null;
  const indexed = pdf.indexed_at !== null;
  const embedded = pdf.embedded_at !== null;

  // Lead with the article's scholarly identity, not the filename.
  const heading = article?.title ?? pdf.filename;
  const meta = [article?.authors, article?.year?.toString(), article?.journal]
    .filter(Boolean)
    .join(" · ");

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.97 }}
    >
      <Card className="p-5">
        <div className="flex items-start gap-4">
          <button
            onClick={onOpen}
            title="Open the PDF"
            className="w-10 h-12 rounded-[var(--radius-xs)] bg-[var(--color-paper-200)] text-[var(--color-paper-600)] hover:bg-[var(--color-accent-100)] hover:text-[var(--color-accent-600)] flex items-center justify-center shrink-0 transition-paper"
          >
            <FileText size={18} strokeWidth={1.5} />
          </button>
          <div className="flex-1 min-w-0">
            <button
              onClick={onOpen}
              title="Open the PDF"
              className="group flex items-start gap-1.5 text-left"
            >
              <span className="font-medium text-[15px] text-[var(--color-ink-900)] leading-snug group-hover:text-[var(--color-accent-700)] group-hover:underline underline-offset-2 decoration-[var(--color-accent-300)]">
                {heading}
              </span>
              <ExternalLink size={12} strokeWidth={1.75} className="mt-1 shrink-0 text-[var(--color-paper-400)] group-hover:text-[var(--color-accent-600)]" />
            </button>
            {meta && (
              <div className="text-[12px] text-[var(--color-paper-600)] mt-0.5">{meta}</div>
            )}
            {article?.abstract_text && (
              <p className="text-[12px] text-[var(--color-paper-500)] leading-[1.6] mt-1.5 line-clamp-3">
                {article.abstract_text}
              </p>
            )}
            {codes.length > 0 && (
              <div className="flex flex-wrap gap-1 mt-2">
                {codes.slice(0, 10).map((c) => (
                  <span key={c} className="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-medium bg-[var(--color-paper-100)] border border-[var(--color-paper-300)] text-[var(--color-paper-600)]">
                    {c}
                  </span>
                ))}
                {codes.length > 10 && (
                  <span className="text-[10px] text-[var(--color-paper-400)] self-center">+{codes.length - 10} more</span>
                )}
              </div>
            )}
            <div className="flex flex-wrap items-center gap-3 mt-2.5 text-[11px] text-[var(--color-paper-500)]">
              <span className="font-mono text-[10px] text-[var(--color-paper-400)] truncate max-w-[220px]" title={pdf.filename}>{pdf.filename}</span>
              {pdf.text_length && (
                <span className="font-mono tabular-nums">{pdf.text_length.toLocaleString()} chars</span>
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
