import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  Ban,
  Check,
  ChevronDown,
  ChevronUp,
  ExternalLink,
  FileText,
  GraduationCap,
  HelpCircle,
  Loader2,
  Plus,
  RefreshCw,
  Sparkles,
  Tag,
  Upload,
  X,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import {
  api,
  type ArticleRow,
  type CodingRow,
  type DimensionInfo,
  type FrameworkCodeDefinition,
  type ProjectRow,
  type Suggestion,
} from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { cn } from "../../lib/utils";

interface Props {
  project: ProjectRow;
  refreshKey: number;
  onChanged: () => void;
}

type Filter = "all" | "uncoded" | "coded" | "with-pdf" | "no-pdf" | "excluded";

const LOAD_LIMIT = 9999;

export function CodingPanel({ project, refreshKey, onChanged }: Props) {
  const isCustom = project.framework === "Custom";

  const [dimensions, setDimensions] = useState<DimensionInfo[]>([]);
  const [codeDefs, setCodeDefs] = useState<FrameworkCodeDefinition[]>([]);
  const [articles, setArticles] = useState<ArticleRow[]>([]);
  const [excludedArticles, setExcludedArticles] = useState<ArticleRow[]>([]);
  const [criteria, setCriteria] = useState<import("../../lib/api").ExclusionCriterionRow[]>([]);
  const [unreviewedCount, setUnreviewedCount] = useState(0);
  const [codings, setCodings] = useState<CodingRow[]>([]);
  const [pdfMap, setPdfMap] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [scanMsg, setScanMsg] = useState<string | null>(null);
  const [showNamingHint, setShowNamingHint] = useState(false);


  const [filter, setFilter] = useState<Filter>("all");
  const [cardIndex, setCardIndex] = useState(0);
  const [abstractExpanded, setAbstractExpanded] = useState(false);

  // Drag-drop state managed at parent level (card remounts under AnimatePresence)
  const [cardDragOver, setCardDragOver] = useState(false);
  const [dropping, setDropping] = useState(false);
  const [pdfError, setPdfError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    const [dims, defs, batch, allArticles, crits] = await Promise.all([
      api.listFrameworkDimensions(),
      api.listFrameworkCodeDefs(),
      api.listCodingBatch(0, LOAD_LIMIT),
      api.listArticles(),
      api.listCriteria(),
    ]);
    setDimensions(dims);
    setCodeDefs(defs);
    setArticles(batch.articles);
    setCodings(batch.codings);
    setPdfMap(batch.pdf_map);
    setExcludedArticles(allArticles.filter((a) => a.status === "excluded"));
    setUnreviewedCount(allArticles.filter((a) => a.status === "retrieved").length);
    setCriteria(crits);
    setLoading(false);
  }, []);

  useEffect(() => {
    reload();
  }, [refreshKey, reload]);

  const codingsByArticle = useMemo(() => {
    const m = new Map<number, CodingRow[]>();
    for (const c of codings) {
      if (!m.has(c.article_id)) m.set(c.article_id, []);
      m.get(c.article_id)!.push(c);
    }
    return m;
  }, [codings]);

  const codedIds = useMemo(() => {
    const s = new Set<number>();
    for (const c of codings) s.add(c.article_id);
    return s;
  }, [codings]);

  const filtered = useMemo(() => {
    if (filter === "excluded") return [];
    if (filter === "coded")    return articles.filter((a) => codedIds.has(a.id));
    if (filter === "uncoded")  return articles.filter((a) => !codedIds.has(a.id));
    if (filter === "with-pdf") return articles.filter((a) => !!pdfMap[String(a.id)]);
    if (filter === "no-pdf")   return articles.filter((a) => !pdfMap[String(a.id)]);
    return articles;
  }, [articles, codedIds, pdfMap, filter]);

  useEffect(() => {
    setCardIndex((i) => Math.min(i, Math.max(0, filtered.length - 1)));
    setAbstractExpanded(false);
  }, [filter, filtered.length]);

  // Keep a ref to current article so the drop handler (closed over at mount) can read it
  const currentArticleRef = useRef<ArticleRow | null>(null);
  const currentArticle = filtered[cardIndex] ?? null;
  useEffect(() => { currentArticleRef.current = currentArticle; }, [currentArticle]);

  // Listen for Tauri native file-drop — the ONLY way to get real file paths from Finder
  // (HTML5 FileList.path is always empty in WKWebView)
  useEffect(() => {
    let unlistenDrop: (() => void) | null = null;
    let unlistenEnter: (() => void) | null = null;
    let unlistenLeave: (() => void) | null = null;

    listen<{ paths: string[] }>("tauri://drag-drop", async (event) => {
      setCardDragOver(false);
      const article = currentArticleRef.current;
      if (!article) return;
      const pdfPath = event.payload.paths.find((p) => p.toLowerCase().endsWith(".pdf"));
      if (!pdfPath) return;
      setDropping(true);
      setPdfError(null);
      try {
        await api.dropPdfForArticle(pdfPath, article.id);
        await reload();
      } catch (e: any) {
        setPdfError(e?.message ?? String(e));
      } finally {
        setDropping(false);
      }
    }).then((fn) => { unlistenDrop = fn; });

    listen("tauri://drag-enter", () => setCardDragOver(true))
      .then((fn) => { unlistenEnter = fn; });
    listen("tauri://drag-leave", () => setCardDragOver(false))
      .then((fn) => { unlistenLeave = fn; });

    return () => {
      unlistenDrop?.();
      unlistenEnter?.();
      unlistenLeave?.();
    };
  }, [reload]);

  const goTo = (i: number) => {
    const next = Math.max(0, Math.min(i, filtered.length - 1));
    if (next !== cardIndex) setAbstractExpanded(false);
    setCardIndex(next);
  };

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement).closest("input,textarea")) return;
      if (e.key === "ArrowLeft") goTo(cardIndex - 1);
      if (e.key === "ArrowRight") goTo(cardIndex + 1);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [cardIndex, filtered.length]);

  const onCodingChanged = async () => {
    onChanged();
    await reload();
  };

  const handleScan = async () => {
    setScanning(true);
    setScanMsg(null);
    try {
      const r = await api.scanWorkspacePdfs();
      if (r.scanned === 0) {
        setScanMsg("No PDFs found in the pdfs/ folder.");
      } else {
        const parts = [];
        if (r.matched > 0)      parts.push(`${r.matched} matched`);
        if (r.already_known > 0) parts.push(`${r.already_known} already known`);
        const unmatched = r.scanned - r.matched - r.already_known;
        if (unmatched > 0)      parts.push(`${unmatched} unmatched`);
        setScanMsg(`${r.scanned} file${r.scanned !== 1 ? "s" : ""} — ${parts.join(", ")}.`);
        if (r.matched > 0) await reload();
      }
    } catch (e: any) {
      setScanMsg(`Scan failed: ${e?.message || String(e)}`);
    } finally {
      setScanning(false);
    }
  };

  const handleOpenPdf = async (pdfPath: string) => {
    setPdfError(null);
    try {
      await api.openPdfPath(pdfPath);
    } catch (e: any) {
      setPdfError(e?.message ?? String(e));
    }
  };

  const handleOpenFallback = async (article: ArticleRow) => {
    if (article.doi) {
      await openUrl(`https://doi.org/${article.doi}`).catch(() => {});
    } else {
      const q = encodeURIComponent(article.title);
      await openUrl(`https://scholar.google.com/scholar?q=${q}`).catch(() => {});
    }
  };

  const pickPdf = async (): Promise<string | null> => {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
      title: "Select PDF for this article",
    });
    if (!selected || Array.isArray(selected)) return null;
    return selected as string;
  };

  const handleBrowsePdf = async (articleId: number) => {
    const path = await pickPdf();
    if (!path) return;
    setDropping(true);
    setPdfError(null);
    try {
      await api.dropPdfForArticle(path, articleId);
      await reload();
    } catch (e: any) {
      setPdfError(e?.message ?? String(e));
    } finally {
      setDropping(false);
    }
  };

  const handleReplacePdf = async (articleId: number) => {
    const path = await pickPdf();
    if (!path) return;
    setDropping(true);
    setPdfError(null);
    try {
      await api.replaceArticlePdf(path, articleId);
      await reload();
    } catch (e: any) {
      setPdfError(e?.message ?? String(e));
    } finally {
      setDropping(false);
    }
  };

  if (!project.framework) {
    return (
      <div className="text-[13px] text-[var(--color-paper-500)]">No framework selected.</div>
    );
  }

  const codedCount = codedIds.size;
  const total = articles.length;
  const pct = total > 0 ? Math.round((codedCount / total) * 100) : 0;
  const withPdfCount = Object.keys(pdfMap).length;

  return (
    <div className="w-full">
      {/* ── Page header ────────────────────────────────────────────────── */}
      <div className="flex items-start justify-between mb-5">
        <div>
          <div className="eyebrow text-[var(--color-paper-500)]">Coding</div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">
            {project.framework}
            <span className="text-[var(--color-paper-400)] font-sans font-normal text-[18px] ml-2">
              · {dimensions.length} dimension{dimensions.length !== 1 ? "s" : ""}
            </span>
          </h2>
          {isCustom && (
            <p className="text-[12px] text-[var(--color-accent-500)] mt-2">
              Manage dimensions and starter codes in the <strong>Organisation</strong> tab.
            </p>
          )}
        </div>

        <div className="flex flex-col items-end gap-1.5">
          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowNamingHint((v) => !v)}
              className="p-1 text-[var(--color-paper-400)] hover:text-[var(--color-ink-600)] transition-paper"
              title="PDF naming convention"
            >
              <HelpCircle size={14} strokeWidth={1.75} />
            </button>
            <Button variant="ghost" size="sm" onClick={handleScan} disabled={scanning}>
              {scanning ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} strokeWidth={2} />}
              Recheck PDFs
            </Button>
          </div>
          {scanMsg && (
            <p className="text-[11px] text-[var(--color-paper-500)] text-right max-w-[260px] leading-snug">
              {scanMsg}
            </p>
          )}
          <AnimatePresence>
            {showNamingHint && (
              <motion.div
                initial={{ opacity: 0, y: -4 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -4 }}
                className="mt-1 p-3 rounded-[var(--radius-sm)] bg-[var(--color-ink-50)] border border-[var(--color-paper-300)] text-[11px] text-[var(--color-paper-700)] max-w-[280px] leading-relaxed"
              >
                <p className="font-semibold text-[var(--color-ink-800)] mb-1">PDF naming convention</p>
                <p>
                  Auto-match uses DOI and title words. To force a match, include{" "}
                  <code className="font-mono bg-[var(--color-paper-200)] px-1 rounded">id{"{N}"}</code>{" "}
                  in the filename, where <em>N</em> is the article ID shown on each card.
                </p>
                <p className="mt-1.5 font-mono text-[10px] text-[var(--color-paper-600)]">
                  Examples: id42_smith2023.pdf · paper-id42.pdf
                </p>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>

      {/* ── Progress bar ───────────────────────────────────────────────── */}
      <div className="mb-4">
        <div className="flex items-center justify-between mb-1.5">
          <span className="text-[12px] text-[var(--color-paper-600)]">
            <span className="font-semibold text-[var(--color-ink-800)]">{codedCount}</span>
            {" "}of{" "}
            <span className="font-semibold text-[var(--color-ink-800)]">{total}</span>
            {" "}articles coded
          </span>
          <span className="text-[11px] font-mono tabular-nums text-[var(--color-paper-500)]">{pct}%</span>
        </div>
        <div className="h-1.5 rounded-full bg-[var(--color-paper-200)] overflow-hidden">
          <motion.div
            className="h-full rounded-full bg-[var(--color-ok-400)]"
            initial={false}
            animate={{ width: `${pct}%` }}
            transition={{ duration: 0.4, ease: "easeOut" }}
          />
        </div>
      </div>

      {/* ── Unreviewed notice ──────────────────────────────────────────── */}
      {!loading && unreviewedCount > 0 && (
        <div className="mb-4 px-3.5 py-2.5 rounded-[var(--radius-sm)] bg-[var(--color-warn-500)]/10 border border-[var(--color-warn-500)]/30 text-[12px] text-[var(--color-warn-600)] flex items-center gap-2">
          <AlertTriangle size={13} strokeWidth={1.75} className="shrink-0" />
          <span>
            <strong>{unreviewedCount}</strong> article{unreviewedCount !== 1 ? "s" : ""} haven't been screened yet — go to <strong>Screening</strong> to include or exclude them.
          </span>
        </div>
      )}

      {/* ── Filter tabs ────────────────────────────────────────────────── */}
      <div className="flex items-center gap-1 flex-wrap mb-5">
        {([
          { key: "all",      label: "Included",  count: total },
          { key: "uncoded",  label: "Uncoded",   count: total - codedCount },
          { key: "coded",    label: "Coded",     count: codedCount },
          { key: "with-pdf", label: "Has PDF",   count: withPdfCount },
          { key: "no-pdf",   label: "No PDF",    count: total - withPdfCount },
        ] as { key: Filter; label: string; count: number }[]).map(({ key, label, count }) => (
          <button
            key={key}
            onClick={() => { setFilter(key); setCardIndex(0); }}
            className={cn(
              "px-3 py-1.5 rounded-full text-[12px] font-medium transition-paper",
              filter === key
                ? "bg-[var(--color-ink-800)] text-[var(--color-paper-50)]"
                : "text-[var(--color-paper-600)] hover:bg-[var(--color-paper-200)]"
            )}
          >
            {label}
            <span className="ml-1.5 tabular-nums text-[var(--color-paper-400)]">{count}</span>
          </button>
        ))}
        {/* Excluded tab — separated visually */}
        {excludedArticles.length > 0 && (
          <>
            <span className="mx-1 text-[var(--color-paper-300)]">|</span>
            <button
              onClick={() => { setFilter("excluded"); setCardIndex(0); }}
              className={cn(
                "px-3 py-1.5 rounded-full text-[12px] font-medium transition-paper",
                filter === "excluded"
                  ? "bg-[var(--color-error-500)] text-white"
                  : "text-[var(--color-error-500)] hover:bg-[var(--color-error-500)]/10"
              )}
            >
              Excluded
              <span className="ml-1.5 tabular-nums opacity-75">{excludedArticles.length}</span>
            </button>
          </>
        )}
      </div>

      {loading && (
        <div className="flex items-center gap-2 text-[13px] text-[var(--color-paper-500)] py-8">
          <Loader2 size={14} className="animate-spin" /> Loading articles…
        </div>
      )}

      {/* Custom framework: no dimensions defined yet */}
      {!loading && isCustom && dimensions.length === 0 && (
        <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-12 text-center mb-6">
          <Tag size={24} strokeWidth={1.5} className="text-[var(--color-paper-300)] mx-auto mb-3" />
          <p className="text-[14px] text-[var(--color-paper-700)] font-medium mb-1">
            No dimensions yet
          </p>
          <p className="text-[13px] text-[var(--color-paper-500)] max-w-sm mx-auto leading-relaxed">
            Go to the <strong>Organisation</strong> tab to define dimensions and starter codes for your custom framework.
          </p>
        </div>
      )}

      {!loading && filtered.length === 0 && filter !== "excluded" && (
        <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-12 text-center">
          <p className="text-[14px] text-[var(--color-paper-700)] font-medium mb-1">
            {filter === "coded"    ? "No coded articles yet." :
             filter === "uncoded"  ? "All articles have been coded." :
             filter === "with-pdf" ? "No articles have a PDF linked." :
             filter === "no-pdf"   ? "All articles have a PDF linked." :
             "No articles to code."}
          </p>
          <p className="text-[13px] text-[var(--color-paper-500)] max-w-md mx-auto leading-relaxed">
            {filter === "all"
              ? "Import records in Stage 1 first, or check the Library tab."
              : `Switch to "All" to see every article.`}
          </p>
        </div>
      )}

      {/* ── Excluded articles list ─────────────────────────────────────── */}
      {!loading && filter === "excluded" && (
        <div className="space-y-2">
          {excludedArticles.length === 0 ? (
            <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-12 text-center">
              <Ban size={24} strokeWidth={1.5} className="text-[var(--color-paper-300)] mx-auto mb-3" />
              <p className="text-[14px] text-[var(--color-paper-700)] font-medium mb-1">No excluded articles</p>
              <p className="text-[13px] text-[var(--color-paper-500)] max-w-md mx-auto leading-relaxed">
                Articles you exclude during screening will appear here.
              </p>
            </div>
          ) : (
            excludedArticles.map((a) => {
              const criterion = criteria.find((c) => c.number === a.exclusion_criterion);
              return (
                <Card key={a.id} className="p-4 border-[var(--color-error-500)]/20 bg-[var(--color-error-500)]/3">
                  <div className="flex items-start gap-3">
                    <div className="font-mono text-[10px] text-[var(--color-paper-400)] tabular-nums w-8 text-right shrink-0 mt-0.5">
                      #{a.id}
                    </div>
                    <div className="flex-1 min-w-0">
                      <h3 className="font-serif text-[15px] leading-snug text-[var(--color-ink-800)]">
                        {a.title}
                      </h3>
                      <div className="mt-1 flex items-baseline gap-3 text-[12px] text-[var(--color-paper-500)]">
                        {a.authors && <span className="line-clamp-1">{a.authors}</span>}
                        {a.year && <span className="font-mono tabular-nums">{a.year}</span>}
                        {a.journal && <span className="italic">{a.journal}</span>}
                      </div>
                    </div>
                    {criterion && (
                      <div className="shrink-0 inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-medium bg-[var(--color-error-500)]/10 text-[var(--color-error-500)] border border-[var(--color-error-500)]/20">
                        <Ban size={10} strokeWidth={2} />
                        #{criterion.number} {criterion.label}
                      </div>
                    )}
                  </div>
                </Card>
              );
            })
          )}
        </div>
      )}

      {!loading && filtered.length > 0 && currentArticle && (
        <>
          {/* ── Navigation + strip ───────────────────────────────────── */}
          <div className="flex items-center gap-3 mb-4">
            <button
              onClick={() => goTo(cardIndex - 1)}
              disabled={cardIndex === 0}
              className={cn(
                "flex items-center gap-1.5 px-3 py-2 rounded-[var(--radius-sm)] border text-[12px] font-medium transition-paper shrink-0",
                cardIndex === 0
                  ? "border-transparent text-[var(--color-paper-300)] cursor-not-allowed"
                  : "border-[var(--color-paper-300)] text-[var(--color-paper-600)] hover:border-[var(--color-ink-400)] hover:text-[var(--color-ink-800)] hover:bg-[var(--color-paper-50)]"
              )}
            >
              <ArrowLeft size={13} strokeWidth={2} /> Prev
            </button>

            <div className="flex-1 overflow-x-auto no-scrollbar">
              <div className="flex gap-1 min-w-max py-0.5">
                {filtered.map((a, i) => {
                  const isCoded   = codedIds.has(a.id);
                  const isCurrent = i === cardIndex;
                  return (
                    <button
                      key={a.id}
                      onClick={() => goTo(i)}
                      title={a.title}
                      className={cn(
                        "h-[22px] rounded-full transition-paper text-[9px] font-mono tabular-nums flex items-center justify-center shrink-0",
                        isCurrent
                          ? "w-9 bg-[var(--color-ink-800)] text-[var(--color-paper-50)]"
                          : isCoded
                          ? "w-5 bg-[var(--color-ok-200)] hover:bg-[var(--color-ok-300)] text-[var(--color-ok-700)]"
                          : "w-5 bg-[var(--color-paper-200)] hover:bg-[var(--color-paper-300)] text-transparent"
                      )}
                    >
                      {isCurrent ? i + 1 : ""}
                    </button>
                  );
                })}
              </div>
            </div>

            <button
              onClick={() => goTo(cardIndex + 1)}
              disabled={cardIndex >= filtered.length - 1}
              className={cn(
                "flex items-center gap-1.5 px-3 py-2 rounded-[var(--radius-sm)] border text-[12px] font-medium transition-paper shrink-0",
                cardIndex >= filtered.length - 1
                  ? "border-transparent text-[var(--color-paper-300)] cursor-not-allowed"
                  : "border-[var(--color-paper-300)] text-[var(--color-paper-600)] hover:border-[var(--color-ink-400)] hover:text-[var(--color-ink-800)] hover:bg-[var(--color-paper-50)]"
              )}
            >
              Next <ArrowRight size={13} strokeWidth={2} />
            </button>
          </div>

          {/* ── Article card ─────────────────────────────────────────── */}
          <AnimatePresence mode="wait">
            <motion.div
              key={currentArticle.id}
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              transition={{ duration: 0.16, ease: [0.2, 0.7, 0.2, 1] }}
            >
              <ArticleCodingCard
                index={cardIndex}
                total={filtered.length}
                article={currentArticle}
                dimensions={dimensions}
                codeDefs={codeDefs}
                codings={codingsByArticle.get(currentArticle.id) ?? []}
                pdfPath={pdfMap[String(currentArticle.id)] ?? null}
                isDragOver={cardDragOver}
                dropping={dropping}
                pdfError={pdfError}
                abstractExpanded={abstractExpanded}
                onToggleAbstract={() => setAbstractExpanded((v) => !v)}
                onOpenPdf={(path) => handleOpenPdf(path)}
                onOpenFallback={() => handleOpenFallback(currentArticle)}
                onBrowsePdf={() => handleBrowsePdf(currentArticle.id)}
                onReplacePdf={() => handleReplacePdf(currentArticle.id)}
                onChanged={onCodingChanged}
              />
            </motion.div>
          </AnimatePresence>
        </>
      )}
    </div>
  );
}

// ── Article card ───────────────────────────────────────────────────────────────

function ArticleCodingCard({
  index,
  total,
  article,
  dimensions,
  codeDefs,
  codings,
  pdfPath,
  isDragOver,
  dropping,
  pdfError,
  abstractExpanded,
  onToggleAbstract,
  onOpenPdf,
  onOpenFallback,
  onBrowsePdf,
  onReplacePdf,
  onChanged,
}: {
  index: number;
  total: number;
  article: ArticleRow;
  dimensions: DimensionInfo[];
  codeDefs: FrameworkCodeDefinition[];
  codings: CodingRow[];
  pdfPath: string | null;
  isDragOver: boolean;
  dropping: boolean;
  pdfError: string | null;
  abstractExpanded: boolean;
  onToggleAbstract: () => void;
  onOpenPdf: (path: string) => void;
  onOpenFallback: () => void;
  onBrowsePdf: () => void;
  onReplacePdf: () => void;
  onChanged: () => Promise<void>;
}) {
  const coded = codings.length > 0;
  // HTML5 drag events only used to prevent default browser drop behavior;
  // actual file paths come from Tauri's tauri://drag-drop event in the parent.
  const handleDragOver = (e: React.DragEvent) => { e.preventDefault(); };
  const handleDrop = (e: React.DragEvent) => { e.preventDefault(); };

  return (
    <Card
      className={cn(
        "relative overflow-hidden transition-paper w-full",
        coded && "ring-1 ring-[var(--color-ok-300)]",
        isDragOver && "ring-2 ring-[var(--color-ink-500)]",
      )}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      {/* Drop overlay */}
      <AnimatePresence>
        {isDragOver && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="absolute inset-0 z-10 flex flex-col items-center justify-center gap-2 bg-[var(--color-paper-50)]/90 backdrop-blur-sm pointer-events-none rounded-[var(--radius-md)]"
          >
            <Upload size={28} strokeWidth={1.5} className="text-[var(--color-ink-600)]" />
            <p className="text-[13px] font-medium text-[var(--color-ink-700)]">Drop PDF to link to this article</p>
          </motion.div>
        )}
        {dropping && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="absolute inset-0 z-10 flex items-center justify-center bg-[var(--color-paper-50)]/80 rounded-[var(--radius-md)]"
          >
            <Loader2 size={20} className="animate-spin text-[var(--color-ink-600)]" />
          </motion.div>
        )}
      </AnimatePresence>

      {/* ── Article header ─────────────────────────────────────────────── */}
      <div className="px-6 pt-5 pb-4">
        {/* Counter + coded badge */}
        <div className="flex items-center gap-2 mb-3">
          <span className="font-mono text-[11px] text-[var(--color-paper-400)] tabular-nums">
            {index + 1} / {total}
          </span>
          {coded && (
            <span className="inline-flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider text-[var(--color-ok-600)] bg-[var(--color-ok-50)] border border-[var(--color-ok-200)] rounded-full px-2 py-0.5">
              <Check size={9} strokeWidth={3} /> Coded
            </span>
          )}
        </div>

        {/* Two-column: article info | PDF zone */}
        <div className="flex gap-5 items-start">
          {/* Left: title + metadata */}
          <div className="flex-1 min-w-0">
            <h3 className="font-serif text-[20px] leading-[1.35] text-[var(--color-ink-900)] mb-2">
              {article.title}
            </h3>
            <div className="space-y-0.5">
              {(article.authors || article.year) && (
                <p className="text-[12px] text-[var(--color-paper-700)]">
                  {article.authors && <span>{article.authors}</span>}
                  {article.authors && article.year && <span className="mx-1.5 text-[var(--color-paper-300)]">·</span>}
                  {article.year && <span className="tabular-nums font-mono">{article.year}</span>}
                </p>
              )}
              {article.journal && (
                <p className="text-[12px] italic text-[var(--color-paper-500)]">{article.journal}</p>
              )}
              {article.doi && (
                <p className="text-[11px] font-mono text-[var(--color-paper-500)]">DOI: {article.doi}</p>
              )}
            </div>
            {/* Abstract toggle */}
            {article.abstract_text && (
              <button
                onClick={onToggleAbstract}
                className="mt-3 text-[11px] text-[var(--color-ink-500)] hover:text-[var(--color-ink-800)] font-medium inline-flex items-center gap-1 transition-paper"
              >
                {abstractExpanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
                {abstractExpanded ? "Hide abstract" : "Show abstract"}
              </button>
            )}
            <AnimatePresence>
              {abstractExpanded && article.abstract_text && (
                <motion.p
                  initial={{ opacity: 0, height: 0 }}
                  animate={{ opacity: 1, height: "auto" }}
                  exit={{ opacity: 0, height: 0 }}
                  className="prose-editorial text-[13px] mt-3 leading-[1.75] text-[var(--color-paper-700)] overflow-hidden"
                >
                  {article.abstract_text}
                </motion.p>
              )}
            </AnimatePresence>
          </div>

          {/* Right: PDF zone */}
          <div className="shrink-0 w-[160px] flex flex-col gap-2">
            {pdfPath ? (
              <>
                <button
                  onClick={() => onOpenPdf(pdfPath)}
                  className="w-full flex flex-col items-center justify-center gap-2 px-4 py-4 rounded-[var(--radius-sm)] bg-[var(--color-ok-50)] border border-[var(--color-ok-200)] hover:bg-[var(--color-ok-100)] transition-paper text-[var(--color-ok-700)]"
                >
                  <FileText size={20} strokeWidth={1.5} />
                  <span className="text-[12px] font-medium">Open PDF</span>
                </button>
                <button
                  onClick={onReplacePdf}
                  className="text-[10px] text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] text-center transition-paper flex items-center justify-center gap-1"
                >
                  <Upload size={10} strokeWidth={1.75} /> Replace PDF
                </button>
              </>
            ) : (
              <>
                <button
                  onClick={onBrowsePdf}
                  className={cn(
                    "w-full flex flex-col items-center justify-center gap-2 px-3 py-4 rounded-[var(--radius-sm)] border border-dashed transition-paper text-center",
                    isDragOver
                      ? "border-[var(--color-ink-500)] bg-[var(--color-paper-100)] text-[var(--color-ink-700)]"
                      : "border-[var(--color-paper-300)] text-[var(--color-paper-400)] hover:border-[var(--color-paper-500)] hover:text-[var(--color-paper-600)] hover:bg-[var(--color-paper-50)]"
                  )}
                >
                  <Upload size={18} strokeWidth={1.5} />
                  <div>
                    <p className="text-[11px] font-medium leading-snug">Drop or click</p>
                    <p className="text-[10px] leading-snug mt-0.5">to attach a PDF</p>
                  </div>
                  <p className="text-[9px] font-mono text-[var(--color-paper-400)] leading-snug">
                    or: id{article.id}_file.pdf
                  </p>
                </button>
                <button
                  onClick={onOpenFallback}
                  className="text-[10px] text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] text-center transition-paper flex items-center justify-center gap-1"
                >
                  {article.doi
                    ? <><ExternalLink size={10} strokeWidth={1.75} /> Open DOI</>
                    : <><GraduationCap size={10} strokeWidth={1.75} /> Google Scholar</>
                  }
                </button>
              </>
            )}
            {pdfError && (
              <p className="text-[10px] text-[var(--color-error-500)] leading-snug text-center">
                {pdfError}
              </p>
            )}
          </div>
        </div>
      </div>

      {/* ── Dimensions ─────────────────────────────────────────────────── */}
      <div className="border-t border-[var(--color-paper-200)] divide-y divide-[var(--color-paper-200)]">
        {dimensions.map((dim) => (
          <DimensionRow
            key={dim.name}
            article={article}
            dimension={dim}
            starterCodes={codeDefs.filter((d) => d.dimension === dim.name)}
            codings={codings.filter((c) => c.dimension === dim.name)}
            onChanged={onChanged}
          />
        ))}
      </div>
    </Card>
  );
}

// ── Dimension row ──────────────────────────────────────────────────────────────

function DimensionRow({
  article,
  dimension,
  starterCodes,
  codings,
  onChanged,
}: {
  article: ArticleRow;
  dimension: DimensionInfo;
  starterCodes: FrameworkCodeDefinition[];
  codings: CodingRow[];
  onChanged: () => Promise<void>;
}) {
  const [adding, setAdding] = useState(false);
  const [newCode, setNewCode] = useState("");
  const [suggesting, setSuggesting] = useState(false);
  const [suggestions, setSuggestions] = useState<Suggestion[] | null>(null);
  const [suggestModel, setSuggestModel] = useState<string | null>(null);
  const [suggestError, setSuggestError] = useState<string | null>(null);

  const addCode = async (
    code: string,
    source: "researcher" | "model-suggested-accepted" | "model-suggested-edited",
    rationale?: string,
    model?: string,
  ) => {
    if (!code.trim()) return;
    setSuggestions(null);
    setNewCode("");
    setAdding(false);
    await api.saveCoding({
      article_id: article.id,
      dimension: dimension.name,
      code: code.trim(),
      rationale: rationale ?? null,
      source,
      model_name: model ?? null,
    });
    await onChanged();
  };

  const removeCode = async (id: number) => {
    await api.deleteCoding(id);
    await onChanged();
  };

  const requestSuggestions = async () => {
    setSuggesting(true);
    setSuggestError(null);
    try {
      const r = await api.suggestCodes(article.id, dimension.name);
      setSuggestions(r.suggestions);
      setSuggestModel(r.model);
    } catch (e: any) {
      setSuggestError(e?.toString() || "Suggestion failed");
    } finally {
      setSuggesting(false);
    }
  };

  return (
    <div className="px-6 py-4 grid grid-cols-[180px_1fr_auto] gap-4 items-start">
      <div className="pt-0.5">
        <span className="text-[12px] font-semibold tracking-tight text-[var(--color-ink-800)]">
          {dimension.name}
        </span>
        {dimension.description && (
          <div className="text-[10px] text-[var(--color-paper-500)] mt-0.5 line-clamp-3 leading-relaxed">
            {dimension.description}
          </div>
        )}
      </div>

      <div className="min-w-0">
        <div className="flex flex-wrap gap-1.5 items-center min-h-[26px]">
          {codings.map((c) => (
            <AppliedCode key={c.id} coding={c} onRemove={() => removeCode(c.id)} />
          ))}
          {codings.length === 0 && !adding && (
            <span className="text-[11px] italic text-[var(--color-paper-400)]">no codes yet</span>
          )}
          {adding && (
            <div className="flex flex-col gap-1.5 mt-1 w-full">
              {starterCodes.length > 0 && (
                <div className="flex flex-wrap gap-1">
                  {starterCodes
                    .filter((s) => !codings.some((c) => c.code === s.code))
                    .map((s) => (
                      <button
                        key={s.id}
                        type="button"
                        onClick={() => addCode(s.code, "researcher")}
                        className="px-2.5 py-0.5 text-[11px] rounded-full border border-dashed border-[var(--color-ink-400)] text-[var(--color-ink-600)] hover:bg-[var(--color-ink-100)] hover:border-[var(--color-ink-600)] transition-paper"
                      >
                        {s.code}
                      </button>
                    ))}
                </div>
              )}
              <input
                autoFocus
                value={newCode}
                onChange={(e) => setNewCode(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") addCode(newCode, "researcher");
                  if (e.key === "Escape") { setAdding(false); setNewCode(""); }
                }}
                placeholder={starterCodes.length > 0 ? "Or type a custom code…" : "Type a code and press Enter…"}
                className="px-2.5 py-0.5 text-[12px] rounded-full bg-white border border-[var(--color-ink-700)] focus:outline-none w-48"
              />
            </div>
          )}
        </div>

        <AnimatePresence>
          {suggestions && (
            <motion.div
              initial={{ opacity: 0, y: 4 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -4 }}
              className="mt-2.5 px-3 py-2.5 rounded-[var(--radius-sm)] bg-[var(--color-accent-50)] border border-[var(--color-accent-100)]"
            >
              <div className="flex items-center gap-1.5 text-[var(--color-accent-700)] text-[10px] font-semibold uppercase tracking-wider mb-2">
                <Sparkles size={10} strokeWidth={2} />
                Suggestions
                {suggestModel && (
                  <span className="font-mono normal-case tracking-normal text-[10px] text-[var(--color-paper-500)] ml-1">
                    · {suggestModel}
                  </span>
                )}
                <button onClick={() => setSuggestions(null)} className="ml-auto p-0.5 hover:bg-[var(--color-accent-100)] rounded-sm">
                  <X size={11} />
                </button>
              </div>
              <div className="space-y-1.5">
                {suggestions.map((s, i) => (
                  <button
                    key={i}
                    onClick={() => addCode(s.code, "model-suggested-accepted", s.rationale ?? undefined, suggestModel ?? undefined)}
                    className="w-full text-left px-2.5 py-1.5 rounded-[var(--radius-xs)] bg-white/60 hover:bg-white border border-transparent hover:border-[var(--color-accent-300)] transition-paper group"
                  >
                    <div className="flex items-center gap-2">
                      <Tag size={10} strokeWidth={2} className="text-[var(--color-accent-500)]" />
                      <span className="text-[12px] font-medium text-[var(--color-ink-900)]">{s.code}</span>
                      {s.is_new && (
                        <span className="text-[9px] font-semibold uppercase tracking-wider px-1 py-px rounded-sm bg-[var(--color-accent-300)] text-white">new</span>
                      )}
                      <Check size={12} className="ml-auto opacity-0 group-hover:opacity-100 text-[var(--color-ok-500)]" />
                    </div>
                    {s.rationale && (
                      <p className="ml-5 mt-0.5 text-[11px] text-[var(--color-paper-600)] italic leading-snug">{s.rationale}</p>
                    )}
                  </button>
                ))}
              </div>
              <p className="mt-2 text-[10px] text-[var(--color-paper-500)] italic">
                Click any suggestion to accept.
              </p>
            </motion.div>
          )}
        </AnimatePresence>

        {suggestError && (
          <p className="mt-2 text-[11px] text-[var(--color-error-500)]">{suggestError}</p>
        )}
      </div>

      <div className="flex items-center gap-0.5 pt-0.5 shrink-0">
        <button
          onClick={() => setAdding(true)}
          className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] text-[var(--color-paper-500)] hover:text-[var(--color-ink-800)] transition-paper"
          title="Add code"
        >
          <Plus size={13} strokeWidth={2} />
        </button>
        <button
          onClick={requestSuggestions}
          disabled={suggesting}
          className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-accent-50)] text-[var(--color-accent-500)] transition-paper disabled:opacity-40"
          title="AI suggestions"
        >
          {suggesting ? <Loader2 size={13} className="animate-spin" /> : <Sparkles size={13} strokeWidth={1.75} />}
        </button>
      </div>
    </div>
  );
}

// ── Applied code chip ──────────────────────────────────────────────────────────

function AppliedCode({ coding, onRemove }: { coding: CodingRow; onRemove: () => void }) {
  const isModel = coding.source.startsWith("model-");
  return (
    <motion.span
      layout
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.9 }}
      className={cn(
        "group inline-flex items-center gap-1.5 pl-2.5 pr-1 py-0.5 rounded-full border text-[12px]",
        isModel
          ? "bg-[var(--color-accent-50)] border-[var(--color-accent-200)] text-[var(--color-accent-700)]"
          : "bg-[var(--color-ink-100)] border-[var(--color-ink-200)] text-[var(--color-ink-800)]",
      )}
      title={coding.rationale ?? undefined}
    >
      {isModel && <Sparkles size={9} strokeWidth={2} />}
      <span className="font-medium">{coding.code}</span>
      <button
        onClick={onRemove}
        className="opacity-0 group-hover:opacity-100 p-0.5 rounded-full hover:bg-black/10 transition-paper"
      >
        <X size={10} strokeWidth={2} />
      </button>
    </motion.span>
  );
}

