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
  Download,
  ExternalLink,
  FileText,
  GraduationCap,
  HelpCircle,
  Loader2,
  MessageSquare,
  Plus,
  RefreshCw,
  Send,
  Sparkles,
  Tag,
  Trash2,
  Upload,
  X,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { open as openDialog, save as saveDialog, confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { listen } from "@tauri-apps/api/event";
import {
  api,
  type AiAdjudication,
  type AiCodingProposal,
  type AskResult,
  type ArticleRow,
  type CodingRow,
  type DimensionInfo,
  type FrameworkCodeDefinition,
  type ProjectRow,
  type Suggestion,
} from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { Markdown } from "../../components/Markdown";
import { cn } from "../../lib/utils";

interface Props {
  project: ProjectRow;
  refreshKey: number;
  onChanged: () => void;
  initialCodeFilter?: { dimension: string; code: string } | null;
  onCodeFilterConsumed?: () => void;
  lastEditedArticleId?: number | null;
  onArticleEdited?: (id: number) => void;
  onOpenAiCodingModal?: () => void;
}

type ToggleFilter = "uncoded" | "coded" | "with-pdf" | "no-pdf";
const OPPOSITES: Record<ToggleFilter, ToggleFilter> = {
  coded: "uncoded", uncoded: "coded",
  "with-pdf": "no-pdf", "no-pdf": "with-pdf",
};

const LOAD_LIMIT = 9999;

export function CodingPanel({ project, refreshKey, onChanged, initialCodeFilter, onCodeFilterConsumed, lastEditedArticleId, onArticleEdited, onOpenAiCodingModal }: Props) {
  const isCustom = project.framework === "Custom";

  const [dimensions, setDimensions] = useState<DimensionInfo[]>([]);
  const [codeDefs, setCodeDefs] = useState<FrameworkCodeDefinition[]>([]);
  const [articles, setArticles] = useState<ArticleRow[]>([]);
  const [criteria, setCriteria] = useState<import("../../lib/api").ExclusionCriterionRow[]>([]);
  const [unreviewedCount, setUnreviewedCount] = useState(0);
  const [codings, setCodings] = useState<CodingRow[]>([]);
  const [pdfMap, setPdfMap] = useState<Record<string, string>>({});
  const [indexedIds, setIndexedIds] = useState<Set<number>>(new Set());
  const [loading, setLoading] = useState(true);
  const [proposals, setProposals] = useState<AiCodingProposal[]>([]);
  const [adjudications, setAdjudications] = useState<AiAdjudication[]>([]);
  const [scanning, setScanning] = useState(false);
  const [scanMsg, setScanMsg] = useState<string | null>(null);
  const [showNamingHint, setShowNamingHint] = useState(false);

  // Multi-select filters
  const [activeFilters, setActiveFilters] = useState<Set<ToggleFilter>>(new Set());
  // Code filter: Set of "dimension::code" strings, OR logic across selected codes
  const [codeFilters, setCodeFilters] = useState<Set<string>>(new Set());

  const [cardIndex, setCardIndex] = useState(0);
  const [abstractExpanded, setAbstractExpanded] = useState(false);

  // Drag-drop state managed at parent level (card remounts under AnimatePresence)
  const [cardDragOver, setCardDragOver] = useState(false);
  const [dropping, setDropping] = useState(false);
  const [pdfError, setPdfError] = useState<string | null>(null);

  const reload = useCallback(async (silent = false) => {
    if (!silent) setLoading(true);
    const [dims, defs, batch, allArticles, crits, idxIds] = await Promise.all([
      api.listFrameworkDimensions(),
      api.listFrameworkCodeDefs(),
      api.listCodingBatch(0, LOAD_LIMIT),
      api.listArticles(),
      api.listCriteria(),
      api.getIndexedArticleIds(),
    ]);
    setDimensions(dims);
    setCodeDefs(defs);
    setArticles(batch.articles);
    setCodings(batch.codings);
    setPdfMap(batch.pdf_map);
    setIndexedIds(new Set(idxIds));
    setUnreviewedCount(allArticles.filter((a) => a.status === "retrieved").length);
    setCriteria(crits);
    // Load proposals + adjudications for all articles in this batch
    const ids = batch.articles.map((a) => a.id);
    const [props, adjs] = ids.length > 0
      ? await Promise.all([
          api.listAiCodingProposalsForArticles(ids),
          api.listAdjudicationsForArticles(ids),
        ])
      : [[], []];
    setProposals(props);
    setAdjudications(adjs);
    if (!silent) setLoading(false);
  }, []);

  // Refresh coding data in place after a single review action (accept / reject /
  // undo) WITHOUT the loading flash or a parent refresh — both unmount the card
  // and collapse the review panel, losing the reviewer's place. Lets the user
  // work through the conflicts one by one.
  const reviewRefresh = useCallback(() => reload(true), [reload]);

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

  const articleIdsWithProposals = useMemo(() => {
    const s = new Set<number>();
    for (const p of proposals) s.add(p.article_id);
    return s;
  }, [proposals]);

  // Consume jump-to-code from OrganizationPanel
  useEffect(() => {
    if (!initialCodeFilter) return;
    const key = `${initialCodeFilter.dimension}::${initialCodeFilter.code}`;
    setCodeFilters(new Set([key]));
    setActiveFilters(new Set());
    setCardIndex(0);
    onCodeFilterConsumed?.();
  }, [initialCodeFilter]);

  // Build per-article code set for fast lookup: articleId → Set<"dim::code">
  const articleCodeSet = useMemo(() => {
    const m = new Map<number, Set<string>>();
    for (const c of codings) {
      if (!m.has(c.article_id)) m.set(c.article_id, new Set());
      m.get(c.article_id)!.add(`${c.dimension}::${c.code}`);
    }
    return m;
  }, [codings]);

  const filtered = useMemo(() => {
    let result = articles;
    if (activeFilters.has("coded"))    result = result.filter((a) => codedIds.has(a.id));
    if (activeFilters.has("uncoded"))  result = result.filter((a) => !codedIds.has(a.id));
    if (activeFilters.has("with-pdf")) result = result.filter((a) => !!pdfMap[String(a.id)]);
    if (activeFilters.has("no-pdf"))   result = result.filter((a) => !pdfMap[String(a.id)]);
    if (codeFilters.size > 0) {
      result = result.filter((a) => {
        const codes = articleCodeSet.get(a.id);
        if (!codes) return false;
        for (const cf of codeFilters) if (codes.has(cf)) return true;
        return false;
      });
    }
    return result;
  }, [articles, codedIds, pdfMap, activeFilters, codeFilters, articleCodeSet]);

  const toggleFilter = (key: ToggleFilter) => {
    setActiveFilters((prev) => {
      const next = new Set(prev);
      if (next.has(key)) { next.delete(key); }
      else { next.delete(OPPOSITES[key]); next.add(key); }
      return next;
    });
    setCardIndex(0);
  };

  const toggleCodeFilter = (key: string) => {
    setCodeFilters((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key); else next.add(key);
      return next;
    });
    setCardIndex(0);
  };

  const clearAllFilters = () => {
    setActiveFilters(new Set());
    setCodeFilters(new Set());
    setCardIndex(0);
  };

  useEffect(() => {
    setCardIndex((i) => Math.min(i, Math.max(0, filtered.length - 1)));
    setAbstractExpanded(false);
  }, [activeFilters, codeFilters, filtered.length]);

  // Keep a ref to current article so the drop handler (closed over at mount) can read it
  const currentArticleRef = useRef<ArticleRow | null>(null);
  const currentArticle = filtered[cardIndex] ?? null;
  useEffect(() => { currentArticleRef.current = currentArticle; }, [currentArticle]);

  // Keep the active number-button visible without forcing the user to scroll
  // the number strip manually when they hit Prev / Next or click somewhere
  // in the middle of the strip.
  const activeNumberRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    activeNumberRef.current?.scrollIntoView({
      behavior: "smooth",
      inline: "center",
      block: "nearest",
    });
  }, [cardIndex]);

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
    if (currentArticle) onArticleEdited?.(currentArticle.id);
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

  const [resetting, setResetting] = useState(false);
  const handleResetCoding = async () => {
    const ok = await confirmDialog(
      "Delete every applied code and all pending AI suggestions across all articles? This cannot be undone.",
      { title: "Reset all coding", kind: "warning" },
    );
    if (!ok) return;
    setResetting(true);
    try {
      await api.resetAllCoding();
      onChanged();
      await reload();
    } catch (e: any) {
      setScanMsg(`Reset failed: ${e?.message || String(e)}`);
    } finally {
      setResetting(false);
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
    <div className="w-full pl-6 pr-4 xl:pl-12 xl:pr-8">
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
        </div>

        <div className="flex flex-col items-end gap-1.5">
          <div className="flex items-center gap-2">
            {onOpenAiCodingModal && (
              <Button variant="ghost" size="sm" onClick={onOpenAiCodingModal}>
                <Sparkles size={13} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                Code with AI Raters
              </Button>
            )}
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
            <Button variant="ghost" size="sm" onClick={handleResetCoding} disabled={resetting}>
              {resetting ? <Loader2 size={13} className="animate-spin" /> : <Trash2 size={13} strokeWidth={2} />}
              Reset coding
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

      {/* ── Proposal review banner ── */}
      {articleIdsWithProposals.size > 0 && (
        <div className="mb-4 flex items-center gap-2.5 px-4 py-2.5 rounded-[var(--radius-sm)] bg-amber-50 border border-amber-200 text-amber-800">
          <AlertTriangle size={13} strokeWidth={2} className="shrink-0" />
          <p className="text-[12px] font-medium">
            {articleIdsWithProposals.size} article{articleIdsWithProposals.size !== 1 ? "s have" : " has"} AI coding suggestions awaiting review
          </p>
        </div>
      )}

      {/* ── Filters ────────────────────────────────────────────────────── */}
      <div className="mb-5 space-y-2">
        {/* Row 1: toggle chips */}
        <div className="flex items-center gap-1 flex-wrap">
          {/* "All" pill — clears all toggles */}
          <button
            onClick={clearAllFilters}
            className={cn(
              "px-3 py-1.5 rounded-full text-[12px] font-medium transition-paper",
              activeFilters.size === 0 && codeFilters.size === 0
                ? "bg-[var(--color-ink-800)] text-white"
                : "text-[var(--color-paper-600)] hover:bg-[var(--color-paper-200)]"
            )}
          >
            All
            <span className="ml-1.5 tabular-nums text-[var(--color-paper-400)]">{total}</span>
          </button>

          {([
            { key: "uncoded"  as ToggleFilter, label: "Uncoded",  count: total - codedCount },
            { key: "coded"    as ToggleFilter, label: "Coded",    count: codedCount },
            { key: "with-pdf" as ToggleFilter, label: "Has PDF",  count: withPdfCount },
            { key: "no-pdf"   as ToggleFilter, label: "No PDF",   count: total - withPdfCount },
          ]).map(({ key, label, count }) => (
            <button
              key={key}
              onClick={() => toggleFilter(key)}
              className={cn(
                "px-3 py-1.5 rounded-full text-[12px] font-medium transition-paper",
                activeFilters.has(key)
                  ? "bg-[var(--color-ink-700)] text-white"
                  : "text-[var(--color-paper-600)] hover:bg-[var(--color-paper-200)] border border-transparent hover:border-[var(--color-paper-300)]"
              )}
            >
              {label}
              <span className={cn("ml-1.5 tabular-nums", activeFilters.has(key) ? "text-white/60" : "text-[var(--color-paper-400)]")}>{count}</span>
            </button>
          ))}

          {/* Clear active filters badge */}
          {(activeFilters.size > 0 || codeFilters.size > 0) && (
            <button
              onClick={clearAllFilters}
              className="ml-1 inline-flex items-center gap-1 px-2 py-1 rounded-full text-[11px] text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] hover:bg-[var(--color-paper-200)] transition-paper"
            >
              <X size={10} strokeWidth={2} /> Clear filters
            </button>
          )}
        </div>

        {/* Row 2: code filters — only shown when codings exist */}
        {codings.length > 0 && (
          <div className="flex items-start gap-2 flex-wrap">
            <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-paper-400)] mt-1.5 shrink-0">
              By code:
            </span>
            <div className="flex flex-wrap gap-1">
              {Array.from(
                codings.reduce((acc, c) => {
                  const key = `${c.dimension}::${c.code}`;
                  if (!acc.has(key)) acc.set(key, { dimension: c.dimension, code: c.code, count: 0 });
                  acc.get(key)!.count++;
                  return acc;
                }, new Map<string, { dimension: string; code: string; count: number }>())
                .values()
              )
                .sort((a, b) => a.code.localeCompare(b.code, undefined, { sensitivity: "base" }))
                .map(({ dimension, code, count }) => {
                  const key = `${dimension}::${code}`;
                  const active = codeFilters.has(key);
                  return (
                    <button
                      key={key}
                      onClick={() => toggleCodeFilter(key)}
                      title={dimension}
                      className={cn(
                        "inline-flex items-center gap-1 pl-2 pr-1.5 py-0.5 rounded-full text-[11px] border transition-paper",
                        active
                          ? "bg-[var(--color-accent-600)] border-[var(--color-accent-600)] text-white"
                          : "bg-[var(--color-paper-50)] border-[var(--color-paper-300)] text-[var(--color-paper-700)] hover:border-[var(--color-accent-400)] hover:text-[var(--color-accent-700)]"
                      )}
                    >
                      <Tag size={9} strokeWidth={2} />
                      <span className="font-medium">{code}</span>
                      <span className={cn("font-mono text-[10px] ml-0.5", active ? "text-white/70" : "text-[var(--color-paper-400)]")}>×{count}</span>
                    </button>
                  );
                })}
            </div>
          </div>
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

      {!loading && filtered.length === 0 && (
        <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-12 text-center">
          <p className="text-[14px] text-[var(--color-paper-700)] font-medium mb-1">
            {codeFilters.size > 0
              ? "No articles match the selected code filter."
              : activeFilters.has("coded")    ? "No coded articles yet."
              : activeFilters.has("uncoded")  ? "All articles have been coded."
              : activeFilters.has("with-pdf") ? "No articles have a PDF linked."
              : activeFilters.has("no-pdf")   ? "All articles have a PDF linked."
              : "No articles to code."}
          </p>
          <p className="text-[13px] text-[var(--color-paper-500)] max-w-md mx-auto leading-relaxed">
            {activeFilters.size === 0 && codeFilters.size === 0
              ? "Screen your articles first — only included articles appear here."
              : "Clear the filters to see all articles."}
          </p>
          {(activeFilters.size > 0 || codeFilters.size > 0) && (
            <button onClick={clearAllFilters} className="mt-3 text-[12px] text-[var(--color-ink-600)] underline underline-offset-2 hover:text-[var(--color-ink-900)]">
              Clear filters
            </button>
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
                "flex items-center gap-1.5 px-3 py-2 rounded-[var(--radius-sm)] border text-[13px] font-semibold transition-paper shrink-0",
                cardIndex === 0
                  ? "border-[var(--color-paper-200)] text-[var(--color-paper-400)] cursor-not-allowed bg-[var(--color-paper-50)]"
                  : "border-[var(--color-paper-400)] text-[var(--color-ink-700)] hover:border-[var(--color-ink-500)] hover:text-[var(--color-ink-900)] hover:bg-[var(--color-paper-100)]"
              )}
            >
              <ArrowLeft size={13} strokeWidth={2.5} /> Prev
            </button>

            {/* Counter — always visible outside the scrollable strip */}
            <span className="shrink-0 text-[13px] font-semibold tabular-nums text-[var(--color-ink-800)] min-w-[3.5rem] text-center">
              {cardIndex + 1} <span className="font-normal text-[var(--color-paper-400)]">/ {filtered.length}</span>
            </span>

            <div className="flex-1 overflow-x-auto no-scrollbar">
              <div className="flex gap-1 min-w-max py-0.5">
                {filtered.map((a, i) => {
                  const isCoded      = codedIds.has(a.id);
                  const isCurrent    = i === cardIndex;
                  const isLastEdited = !isCurrent && a.id === lastEditedArticleId;
                  return (
                    <button
                      key={a.id}
                      ref={isCurrent ? activeNumberRef : undefined}
                      onClick={() => goTo(i)}
                      title={`#${i + 1} — ${a.title}${isLastEdited ? " (last edited)" : ""}`}
                      className={cn(
                        "h-[24px] min-w-[28px] px-1.5 rounded-full transition-paper text-[11px] font-medium tabular-nums flex items-center justify-center shrink-0",
                        isCurrent
                          ? "bg-[var(--color-ink-800)] text-white ring-2 ring-[var(--color-ink-800)] ring-offset-1"
                          : isLastEdited
                          ? "bg-[var(--color-accent-100)] text-[var(--color-accent-700)] border border-[var(--color-accent-300)] hover:bg-[var(--color-accent-200)] ring-1 ring-[var(--color-accent-400)] ring-offset-1"
                          : isCoded
                          ? "bg-[var(--color-ok-100)] text-[var(--color-ok-700)] border border-[var(--color-ok-300)] hover:bg-[var(--color-ok-200)]"
                          : "bg-[var(--color-paper-100)] text-[var(--color-paper-600)] border border-[var(--color-paper-300)] hover:bg-[var(--color-paper-200)] hover:text-[var(--color-ink-700)]"
                      )}
                    >
                      {i + 1}
                    </button>
                  );
                })}
              </div>
            </div>

            <button
              onClick={() => goTo(cardIndex + 1)}
              disabled={cardIndex >= filtered.length - 1}
              className={cn(
                "flex items-center gap-1.5 px-3 py-2 rounded-[var(--radius-sm)] border text-[13px] font-semibold transition-paper shrink-0",
                cardIndex >= filtered.length - 1
                  ? "border-[var(--color-paper-200)] text-[var(--color-paper-400)] cursor-not-allowed bg-[var(--color-paper-50)]"
                  : "border-[var(--color-paper-400)] text-[var(--color-ink-700)] hover:border-[var(--color-ink-500)] hover:text-[var(--color-ink-900)] hover:bg-[var(--color-paper-100)]"
              )}
            >
              Next <ArrowRight size={13} strokeWidth={2.5} />
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
                criteria={criteria.filter((c) => c.number > 1)}
                pdfPath={pdfMap[String(currentArticle.id)] ?? null}
                isIndexed={indexedIds.has(currentArticle.id)}
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
                onReviewed={reviewRefresh}
                onExcluded={async () => {
                  await onCodingChanged();
                  // stay on same index — the excluded article drops out of the list,
                  // so the next article slides into this position automatically
                }}
                proposals={proposals.filter((p) => p.article_id === currentArticle.id)}
                adjudications={adjudications.filter((a) => a.article_id === currentArticle.id)}
                hasProposals={articleIdsWithProposals.has(currentArticle.id)}
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
  criteria,
  pdfPath,
  isIndexed,
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
  onReviewed,
  onExcluded,
  proposals,
  adjudications,
  hasProposals,
}: {
  index: number;
  total: number;
  article: ArticleRow;
  dimensions: DimensionInfo[];
  codeDefs: FrameworkCodeDefinition[];
  codings: CodingRow[];
  criteria: import("../../lib/api").ExclusionCriterionRow[];
  pdfPath: string | null;
  isIndexed: boolean;
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
  onReviewed: () => Promise<void>;
  onExcluded: () => Promise<void>;
  proposals: AiCodingProposal[];
  adjudications: AiAdjudication[];
  hasProposals: boolean;
}) {
  const [excludeOpen, setExcludeOpen] = useState(false);
  const [inclusionNoteOpen, setInclusionNoteOpen] = useState(false);
  const [excluding, setExcluding] = useState(false);
  const [chatOpen, setChatOpen] = useState(false);
  const [chatInput, setChatInput] = useState("");
  const [chatHistory, setChatHistory] = useState<{ q: string; result: AskResult | null; error: string | null }[]>([]);
  const [chatBusy, setChatBusy] = useState(false);
  const chatInputRef = useRef<HTMLTextAreaElement>(null);
  const chatScrollRef = useRef<HTMLDivElement>(null);
  const coded = codings.length > 0;

  // Auto-scroll to bottom whenever history grows or a pending entry resolves
  useEffect(() => {
    if (chatScrollRef.current) {
      chatScrollRef.current.scrollTop = chatScrollRef.current.scrollHeight;
    }
  }, [chatHistory]);

  const sendChat = async () => {
    const q = chatInput.trim();
    if (!q || chatBusy) return;
    setChatInput("");
    setChatBusy(true);
    const entry = { q, result: null as AskResult | null, error: null as string | null };
    setChatHistory((h) => [...h, entry]);
    try {
      const result = await api.askArticle(article.id, q);
      setChatHistory((h) => h.map((e, i) => i === h.length - 1 ? { ...e, result } : e));
    } catch (err: any) {
      setChatHistory((h) => h.map((e, i) => i === h.length - 1 ? { ...e, error: err?.message ?? String(err) } : e));
    } finally {
      setChatBusy(false);
    }
  };

  const exportChat = async () => {
    if (chatHistory.length === 0) return;

    // Build citation slug: firstAuthorLastName[_etal]_year
    const buildCitationSlug = (): string => {
      const parts: string[] = [];
      if (article.authors) {
        // Authors may be "Last, First; Last, First" or "First Last, First Last"
        const authorList = article.authors.split(/[;]/).map((s) => s.trim()).filter(Boolean);
        const first = authorList[0];
        // If "Last, First" format — last name is before the comma
        // If "First Last" format — last name is the last token
        let lastName = first.includes(",")
          ? first.split(",")[0].trim()
          : first.split(/\s+/).slice(-1)[0];
        lastName = lastName.replace(/[^a-z0-9]/gi, "").toLowerCase();
        if (lastName) parts.push(lastName);
        if (authorList.length > 1) parts.push("etal");
      }
      if (article.year) parts.push(String(article.year));
      return parts.join("_") || "unknown";
    };

    const now = new Date();
    const dt = [
      now.getFullYear(),
      String(now.getMonth() + 1).padStart(2, "0"),
      String(now.getDate()).padStart(2, "0"),
      "_",
      String(now.getHours()).padStart(2, "0"),
      String(now.getMinutes()).padStart(2, "0"),
      String(now.getSeconds()).padStart(2, "0"),
    ].join("");

    const slug = buildCitationSlug();
    const defaultName = `${slug}_${dt}.md`;

    const lines: string[] = [
      `# Article chat — ${article.title}`,
      `*Exported from Reeves · ${now.toLocaleString()}*`,
      "",
    ];
    for (const entry of chatHistory) {
      lines.push(`**Q:** ${entry.q}`, "");
      if (entry.result) {
        lines.push(`**A:** ${entry.result.answer}`, "");
      } else if (entry.error) {
        lines.push(`**Error:** ${entry.error}`, "");
      }
    }

    const path = await saveDialog({
      defaultPath: defaultName,
      filters: [{ name: "Markdown", extensions: ["md"] }],
      title: "Export article chat",
    });
    if (!path) return;
    await writeTextFile(path, lines.join("\n"));
  };

  const exclude = async (criterionNumber: number) => {
    setExcluding(true);
    try {
      await api.excludeArticle(article.id, criterionNumber);
      await onExcluded();
    } finally {
      setExcluding(false);
      setExcludeOpen(false);
    }
  };

  // HTML5 drag events only used to prevent default browser drop behavior;
  // actual file paths come from Tauri's tauri://drag-drop event in the parent.
  const handleDragOver = (e: React.DragEvent) => { e.preventDefault(); };
  const handleDrop = (e: React.DragEvent) => { e.preventDefault(); };

  return (
    <Card
      className={cn(
        "relative overflow-hidden transition-paper w-full",
        coded && !hasProposals && "ring-1 ring-[var(--color-ok-300)]",
        hasProposals && "ring-1 ring-amber-300",
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
                <button
                  onClick={() => openUrl(`https://doi.org/${article.doi}`).catch(() => {})}
                  className="text-left text-[11px] font-mono text-[var(--color-paper-500)] hover:text-[var(--color-accent-600)] hover:underline underline-offset-2 transition-paper"
                >
                  DOI: {article.doi}
                </button>
              )}
            </div>
              {/* Inclusion note — opens modal with full AI rationale or human-screen date */}
              {(article.ai_rationale || article.screened_at) && (
                <button
                  onClick={() => setInclusionNoteOpen(true)}
                  className="mt-2 text-[11px] text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)] font-medium transition-paper"
                >
                  Inclusion note
                </button>
              )}
              <AnimatePresence>
                {inclusionNoteOpen && (
                  <motion.div
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.12 }}
                    className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/30 backdrop-blur-sm"
                    onClick={() => setInclusionNoteOpen(false)}
                  >
                    <motion.div
                      initial={{ opacity: 0, scale: 0.97, y: 6 }}
                      animate={{ opacity: 1, scale: 1, y: 0 }}
                      exit={{ opacity: 0, scale: 0.97, y: 6 }}
                      transition={{ duration: 0.15 }}
                      className="w-full max-w-md bg-white rounded-[var(--radius-lg)] border border-[var(--color-paper-300)] shadow-[var(--shadow-paper-lg)] overflow-hidden"
                      onClick={(e) => e.stopPropagation()}
                    >
                      <div className="flex items-center gap-2 px-5 py-3.5 border-b border-[var(--color-paper-200)]">
                        {article.ai_rationale
                          ? <Sparkles size={14} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                          : <Check size={14} strokeWidth={2} className="text-[var(--color-ok-600)]" />
                        }
                        <span className="text-[13px] font-semibold text-[var(--color-ink-900)] flex-1">
                          {article.ai_rationale ? "AI screener — inclusion note" : "Inclusion note"}
                        </span>
                        <button
                          onClick={() => setInclusionNoteOpen(false)}
                          className="p-1 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] text-[var(--color-paper-500)] transition-paper"
                        >
                          <X size={13} strokeWidth={2} />
                        </button>
                      </div>
                      <div className="px-5 py-4">
                        {article.ai_rationale ? (
                          <p className="text-[13px] text-[var(--color-ink-800)] leading-relaxed">
                            {article.ai_rationale}
                            {article.ai_confidence !== null && (
                              <span className="ml-2 text-[11px] text-[var(--color-paper-400)] not-italic font-mono">
                                ({Math.round(article.ai_confidence * 100)}% confidence)
                              </span>
                            )}
                          </p>
                        ) : (
                          <p className="text-[13px] text-[var(--color-paper-600)]">
                            Included manually
                            {article.screened_at && (
                              <span className="ml-1.5 font-mono text-[12px] text-[var(--color-paper-400)]">
                                · {new Date(article.screened_at).toLocaleDateString()}
                              </span>
                            )}
                          </p>
                        )}
                      </div>
                    </motion.div>
                  </motion.div>
                )}
              </AnimatePresence>
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
          <div className="shrink-0 w-[130px] xl:w-[160px] flex flex-col gap-2">
            {pdfPath ? (
              <>
                <button
                  onClick={() => onOpenPdf(pdfPath)}
                  className="w-full flex flex-col items-center justify-center gap-2 px-4 py-4 rounded-[var(--radius-sm)] bg-[var(--color-ok-50)] border border-[var(--color-ok-200)] hover:bg-[var(--color-ok-100)] transition-paper text-[var(--color-ok-700)]"
                >
                  <FileText size={20} strokeWidth={1.5} />
                  <span className="text-[12px] font-medium">Open PDF</span>
                </button>
                {!isIndexed && (
                  <IndexForChatButton
                    articleId={article.id}
                    onIndexed={onChanged}
                  />
                )}
                {isIndexed && (
                  <button
                    onClick={() => { setChatOpen((v) => !v); setTimeout(() => chatInputRef.current?.focus(), 80); }}
                    className={cn(
                      "w-full flex items-center justify-center gap-1.5 px-3 py-2 rounded-[var(--radius-sm)] border text-[11px] font-medium transition-paper",
                      chatOpen
                        ? "bg-[var(--color-accent-600)] border-[var(--color-accent-600)] text-white"
                        : "bg-[var(--color-accent-50)] border-[var(--color-accent-200)] text-[var(--color-accent-700)] hover:bg-[var(--color-accent-100)]"
                    )}
                  >
                    <MessageSquare size={12} strokeWidth={1.75} />
                    Chat
                  </button>
                )}
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

      {/* ── Article chat panel ─────────────────────────────────────────── */}
      <AnimatePresence>
        {chatOpen && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.2 }}
            className="overflow-hidden border-t border-[var(--color-accent-200)] bg-[var(--color-accent-50)]"
          >
            <div className="px-6 py-4">
              <div className="flex items-center gap-2 mb-3">
                <MessageSquare size={13} strokeWidth={1.75} className="text-[var(--color-accent-600)]" />
                <span className="text-[12px] font-semibold text-[var(--color-accent-800)]">Chat with this article</span>
                <span className="text-[10px] text-[var(--color-accent-500)] ml-1">— scoped to this PDF only</span>
                {chatHistory.length > 0 && (
                  <button
                    onClick={exportChat}
                    className="ml-auto flex items-center gap-1 px-2 py-0.5 rounded text-[10px] text-[var(--color-accent-600)] hover:bg-[var(--color-accent-100)] transition-paper"
                    title="Export conversation as Markdown"
                  >
                    <Download size={10} strokeWidth={2} /> Export
                  </button>
                )}
              </div>

              {/* Chat history */}
              {chatHistory.length > 0 && (
                <div ref={chatScrollRef} className="mb-3 space-y-3 max-h-64 overflow-y-auto pr-1 scroll-smooth">
                  {chatHistory.map((entry, i) => (
                    <div key={i} className="space-y-2">
                      <div className="flex items-start gap-2">
                        <span className="shrink-0 text-[10px] font-semibold uppercase tracking-wider text-[var(--color-accent-500)] mt-0.5 w-4 text-right">Q</span>
                        <p className="text-[12px] text-[var(--color-ink-800)] leading-snug">{entry.q}</p>
                      </div>
                      {entry.result && (
                        <div className="flex items-start gap-2">
                          <span className="shrink-0 text-[10px] font-semibold uppercase tracking-wider text-[var(--color-accent-600)] mt-0.5 w-4 text-right">A</span>
                          <div className="flex-1 min-w-0 text-[12px] text-[var(--color-ink-700)] leading-relaxed prose-editorial">
                            <Markdown>{entry.result.answer}</Markdown>
                          </div>
                        </div>
                      )}
                      {entry.error && (
                        <div className="flex items-start gap-2">
                          <span className="shrink-0 text-[10px] font-semibold uppercase tracking-wider text-amber-800 mt-0.5 w-4 text-right">!</span>
                          <p className="text-[12px] text-amber-900">{entry.error}</p>
                        </div>
                      )}
                      {!entry.result && !entry.error && (
                        <div className="flex items-start gap-2">
                          <span className="shrink-0 w-4" />
                          <Loader2 size={13} className="animate-spin text-[var(--color-accent-500)] mt-0.5" />
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}

              {/* Input row */}
              <div className="flex gap-2 items-end">
                <textarea
                  ref={chatInputRef}
                  value={chatInput}
                  onChange={(e) => setChatInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendChat(); }
                  }}
                  placeholder="Ask about this article… (Enter to send)"
                  rows={2}
                  className="flex-1 resize-none px-3 py-2 rounded-[var(--radius-sm)] border border-[var(--color-accent-200)] bg-white text-[12px] text-[var(--color-ink-800)] placeholder:text-[var(--color-paper-400)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-400)] transition-paper"
                />
                <button
                  onClick={sendChat}
                  disabled={!chatInput.trim() || chatBusy}
                  className="shrink-0 flex items-center justify-center w-9 h-9 rounded-[var(--radius-sm)] bg-[var(--color-accent-500)] text-white hover:bg-[var(--color-accent-600)] disabled:opacity-40 transition-paper"
                >
                  {chatBusy ? <Loader2 size={13} className="animate-spin" /> : <Send size={13} strokeWidth={2} />}
                </button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

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
            onReviewed={onReviewed}
            proposals={proposals.filter((p) => p.dimension === dim.name)}
            adjudications={adjudications.filter((a) => a.dimension === dim.name)}
          />
        ))}
      </div>

      {/* ── Exclude footer ─────────────────────────────────────────────── */}
      {criteria.length > 0 && (
        <div className="border-t border-[var(--color-paper-200)]">
          <button
            onClick={() => setExcludeOpen((v) => !v)}
            className="w-full flex items-center gap-2 px-6 py-3 text-[11px] font-medium text-[var(--color-paper-500)] hover:text-[var(--color-error-600)] hover:bg-[var(--color-error-500)]/5 transition-paper"
          >
            <Ban size={12} strokeWidth={1.75} />
            Exclude this article
            {excludeOpen ? <ChevronUp size={12} className="ml-auto" /> : <ChevronDown size={12} className="ml-auto" />}
          </button>
          <AnimatePresence>
            {excludeOpen && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: "auto" }}
                exit={{ opacity: 0, height: 0 }}
                className="overflow-hidden"
              >
                <div className="px-6 pb-4 flex flex-wrap gap-1.5">
                  {criteria.map((c) => (
                    <button
                      key={c.id}
                      onClick={() => exclude(c.number)}
                      disabled={excluding}
                      title={c.rationale ?? c.label}
                      className="px-2.5 py-1 text-[11px] rounded-full border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] hover:bg-[var(--color-error-500)]/5 hover:border-[var(--color-error-500)]/40 hover:text-[var(--color-error-500)] text-[var(--color-paper-700)] disabled:opacity-50 transition-paper"
                    >
                      #{c.number} {c.label}
                    </button>
                  ))}
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      )}
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
  onReviewed,
  proposals,
  adjudications,
}: {
  article: ArticleRow;
  dimension: DimensionInfo;
  starterCodes: FrameworkCodeDefinition[];
  codings: CodingRow[];
  onChanged: () => Promise<void>;
  onReviewed: () => Promise<void>;
  proposals: AiCodingProposal[];
  adjudications: AiAdjudication[];
}) {
  const [adding, setAdding] = useState(false);
  const [proposalReviewOpen, setProposalReviewOpen] = useState(false);
  const [acceptingId, setAcceptingId] = useState<number | null>(null);
  const [rejectingId, setRejectingId] = useState<number | null>(null);
  const [undoingId, setUndoingId] = useState<number | null>(null);
  const [expandedAdjId, setExpandedAdjId] = useState<number | null>(null);
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
    <div className="px-6 py-4 grid grid-cols-[180px_1fr] gap-4 items-start">
      <div className="pt-0.5">
        <div className="flex items-center gap-1">
          <span className="text-[12px] font-semibold tracking-tight text-[var(--color-ink-800)]">
            {dimension.name}
          </span>
          {proposals.length > 0 && (
            <button
              onClick={() => setProposalReviewOpen((v) => !v)}
              className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-full text-[10px] font-semibold bg-amber-100 border border-amber-300 text-amber-700 hover:bg-amber-200 transition-paper"
              title="AI coding suggestions awaiting review"
            >
              <AlertTriangle size={9} strokeWidth={2.5} />
              {proposals.length}
            </button>
          )}
          {adjudications.length > 0 && (
            <button
              onClick={() => setProposalReviewOpen((v) => !v)}
              className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-full text-[10px] font-semibold bg-[var(--color-ok-100)] border border-[var(--color-ok-300)] text-[var(--color-ok-700)] hover:bg-[var(--color-ok-200)] transition-paper"
              title="Disagreements auto-resolved by the AI adjudicator"
            >
              <Sparkles size={9} strokeWidth={2.5} />
              {adjudications.length}
            </button>
          )}
          <button
            onClick={() => setAdding(true)}
            className="p-1 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] text-[var(--color-paper-500)] hover:text-[var(--color-ink-800)] transition-paper"
            title="Add code"
          >
            <Plus size={12} strokeWidth={2} />
          </button>
          <button
            onClick={requestSuggestions}
            disabled={suggesting}
            className="p-1 rounded-[var(--radius-xs)] hover:bg-[var(--color-accent-50)] text-[var(--color-accent-500)] transition-paper disabled:opacity-40"
            title="AI suggestions"
          >
            {suggesting ? <Loader2 size={12} className="animate-spin" /> : <Sparkles size={12} strokeWidth={1.75} />}
          </button>
        </div>
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

        {/* Proposal review panel */}
        <AnimatePresence>
          {proposalReviewOpen && (proposals.length > 0 || adjudications.length > 0) && (
            <motion.div
              initial={{ opacity: 0, y: 4 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -4 }}
              className="mt-2.5 rounded-[var(--radius-sm)] border border-amber-200 bg-amber-50 overflow-hidden"
            >
              <div className="flex items-center gap-1.5 px-3 py-2 border-b border-amber-200">
                <AlertTriangle size={11} strokeWidth={2} className="text-amber-600" />
                <span className="text-[11px] font-semibold text-amber-800 uppercase tracking-wider">
                  AI suggestions to review
                </span>
                <button
                  onClick={() => setProposalReviewOpen(false)}
                  className="ml-auto p-0.5 hover:bg-amber-100 rounded-sm"
                >
                  <X size={11} className="text-amber-600" />
                </button>
              </div>

              {/* Group by rater */}
              {[1, 2].map((rater) => {
                const raterProposals = proposals.filter((p) => p.rater === rater);
                if (raterProposals.length === 0) return null;
                return (
                  <div key={rater} className="px-3 py-2.5 border-b border-amber-100 last:border-b-0">
                    <p className="text-[10px] font-semibold uppercase tracking-wider text-amber-600 mb-1.5">
                      Rater {rater}
                      {raterProposals[0].model_name && (
                        <span className="ml-1 font-mono font-normal normal-case tracking-normal text-amber-500">
                          · {raterProposals[0].model_name}
                        </span>
                      )}
                    </p>
                    <div className="space-y-1.5">
                      {raterProposals.map((p) => (
                        <div key={p.id} className="flex items-start gap-2 rounded-[var(--radius-xs)] bg-white/60 border border-amber-100 px-2.5 py-1.5">
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2 flex-wrap">
                              <span className="text-[12px] font-medium text-[var(--color-ink-900)]">{p.code}</span>
                              <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-paper-500)]">
                                Rater {p.rater}
                              </span>
                            </div>
                            {p.rationale && (
                              <p className="text-[11px] text-[var(--color-paper-600)] italic leading-snug mt-0.5">
                                {p.rationale}
                              </p>
                            )}
                          </div>
                          <div className="flex items-center gap-1 shrink-0">
                            <button
                              onClick={async () => {
                                setAcceptingId(p.id);
                                try {
                                  await api.acceptAiCodingProposal(p.id);
                                  await onReviewed();
                                } finally {
                                  setAcceptingId(null);
                                }
                              }}
                              disabled={acceptingId === p.id || rejectingId === p.id}
                              className="px-2 py-0.5 text-[10px] font-semibold rounded-full bg-[var(--color-ok-100)] border border-[var(--color-ok-300)] text-[var(--color-ok-700)] hover:bg-[var(--color-ok-200)] disabled:opacity-40 transition-paper"
                            >
                              {acceptingId === p.id ? <Loader2 size={9} className="animate-spin" /> : "Accept"}
                            </button>
                            <button
                              onClick={async () => {
                                setRejectingId(p.id);
                                try {
                                  await api.rejectAiCodingProposal(p.id);
                                  await onReviewed();
                                } finally {
                                  setRejectingId(null);
                                }
                              }}
                              disabled={acceptingId === p.id || rejectingId === p.id}
                              className="px-2 py-0.5 text-[10px] font-semibold rounded-full bg-white border border-[var(--color-paper-300)] text-[var(--color-paper-600)] hover:bg-[var(--color-paper-100)] disabled:opacity-40 transition-paper"
                            >
                              {rejectingId === p.id ? <Loader2 size={9} className="animate-spin" /> : "Reject"}
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                );
              })}

              {/* Resolved by the independent AI adjudicator — already applied, opt-out */}
              {adjudications.length > 0 && (
                <div className="px-3 py-2.5 bg-[var(--color-ok-50)] border-t border-[var(--color-ok-200)]">
                  <p className="flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider text-[var(--color-ok-700)] mb-1.5">
                    <Sparkles size={10} strokeWidth={2.5} />
                    Resolved by AI adjudicator
                  </p>
                  <div className="space-y-1.5">
                    {adjudications.map((a) => {
                      const expanded = expandedAdjId === a.id;
                      return (
                        <div key={a.id} className="rounded-[var(--radius-xs)] bg-white/70 border border-[var(--color-ok-200)] px-2.5 py-1.5">
                          <div className="flex items-center gap-2">
                            <div className="flex-1 min-w-0 flex items-center gap-1.5 flex-wrap">
                              {/* Green bubble — click to reveal why it was merged */}
                              <button
                                onClick={() => setExpandedAdjId(expanded ? null : a.id)}
                                className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[12px] font-medium bg-[var(--color-ok-100)] border border-[var(--color-ok-300)] text-[var(--color-ok-700)] hover:bg-[var(--color-ok-200)] transition-paper"
                                title="Show why these were merged"
                              >
                                {a.canonical_code}
                                {a.adjudicator_rationale && (
                                  expanded
                                    ? <ChevronUp size={10} strokeWidth={2.5} />
                                    : <ChevronDown size={10} strokeWidth={2.5} />
                                )}
                              </button>
                              <span className="text-[10px] text-[var(--color-paper-500)] truncate">
                                merged “{a.label1}” + “{a.label2}”
                              </span>
                            </div>
                            <button
                              onClick={async () => {
                                setUndoingId(a.id);
                                try {
                                  await api.undoAdjudication(a.id);
                                  await onReviewed();
                                } finally {
                                  setUndoingId(null);
                                }
                              }}
                              disabled={undoingId === a.id}
                              className="shrink-0 px-2 py-0.5 text-[10px] font-semibold rounded-full bg-white border border-[var(--color-paper-300)] text-[var(--color-paper-600)] hover:bg-[var(--color-paper-100)] disabled:opacity-40 transition-paper"
                              title="Undo this merge and restore both original suggestions for manual review"
                            >
                              {undoingId === a.id ? <Loader2 size={9} className="animate-spin" /> : "Undo"}
                            </button>
                          </div>
                          {expanded && a.adjudicator_rationale && (
                            <p className="text-[11px] text-[var(--color-paper-600)] italic leading-snug mt-1.5">
                              {a.adjudicator_rationale}
                            </p>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </motion.div>
          )}
        </AnimatePresence>
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

// ── IndexForChatButton ─────────────────────────────────────────────────────────

function IndexForChatButton({
  articleId,
  onIndexed,
}: {
  articleId: number;
  onIndexed: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleIndex = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.indexArticle(articleId);
      onIndexed();
    } catch (e: unknown) {
      const msg: string = (e as { message?: string })?.message ?? String(e);
      setError(
        msg.includes("embedding model")
          ? "Pick an embedding model in Settings first."
          : msg.includes("Connection refused") || msg.includes("error sending request")
          ? "Could not reach LM Studio — check Settings."
          : msg.includes("no linked PDF")
          ? "This article has no PDF attached yet."
          : "Indexing failed — try again.",
      );
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <button
        onClick={handleIndex}
        disabled={busy}
        title="Chunk and embed this PDF so you can chat with the full text."
        className={cn(
          "w-full flex items-center justify-center gap-1.5 px-3 py-2 rounded-[var(--radius-sm)] border text-[11px] font-medium transition-paper",
          busy
            ? "bg-[var(--color-paper-100)] border-[var(--color-paper-300)] text-[var(--color-paper-500)] cursor-wait"
            : "bg-[var(--color-paper-50)] border-[var(--color-paper-300)] text-[var(--color-ink-700)] hover:bg-white hover:border-[var(--color-paper-500)]",
        )}
      >
        {busy ? (
          <>
            <Loader2 size={12} className="animate-spin" />
            Indexing…
          </>
        ) : (
          <>
            <Sparkles size={12} strokeWidth={1.75} />
            Index for chat
          </>
        )}
      </button>
      {error && (
        <p className="text-[10px] text-[var(--color-warn-600)] leading-snug px-1">{error}</p>
      )}
    </>
  );
}

