import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  Bot,
  Check,
  ChevronDown,
  ChevronUp,
  Filter,
  Loader2,
  RotateCcw,
  Sparkles,
  X,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import {
  api,
  type AiScreeningProgress,
  type AiScreeningResult,
  type ArticleRow,
  type ExclusionCriterionRow,
  type JournalRow,
  type ProjectRow,
  type ScreeningTab,
} from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { type AiScreeningHook } from "../../components/AiScreeningModal";
import { cn } from "../../lib/utils";

interface Props {
  project: ProjectRow;
  refreshKey: number;
  onChanged: () => void;
  onOpenAiModal: () => void;
}

const BATCH_SIZE = 20;
const TABS: { id: ScreeningTab; label: string }[] = [
  { id: "pending", label: "Pending Screening" },
  { id: "included", label: "Included" },
  { id: "excluded", label: "Excluded" },
];

// ABS: higher index = higher quality
const ABS_ORDER = ["1", "2", "3", "4", "4*"];
// ABDC: higher index = higher quality
const ABDC_ORDER = ["C", "B", "A", "A*"];

function parseJournalQuality(raw: string | null) {
  try {
    return raw ? (JSON.parse(raw) as { abs_min?: string; abdc_min?: string; if_min?: number }) : {};
  } catch {
    return {};
  }
}

function meetsAbsThreshold(rating: string | null | undefined, min: string): boolean {
  if (!rating) return false;
  return ABS_ORDER.indexOf(rating) >= ABS_ORDER.indexOf(min);
}

function meetsAbdcThreshold(rating: string | null | undefined, min: string): boolean {
  if (!rating) return false;
  return ABDC_ORDER.indexOf(rating) >= ABDC_ORDER.indexOf(min);
}

function meetsIfThreshold(impactFactor: number | null | undefined, min: number): boolean {
  if (impactFactor == null) return false;
  return impactFactor >= min;
}

/** Case-insensitive journal name match, ignoring leading "The " */
function normalizeJournal(name: string) {
  return name
    .toLowerCase()
    .replace(/^the\s+/, "")
    .replace(/&/g, "and")
    .replace(/[^a-z0-9\s]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function useAiScreening(): AiScreeningHook {
  const [step, setStep] = useState<AiScreeningHook["step"]>("idle");
  const [progress, setProgress] = useState<AiScreeningProgress | null>(null);
  const [result, setResult] = useState<AiScreeningResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isPaused, setIsPaused] = useState(false);
  const unlistenRef = useRef<(() => void) | null>(null);

  const start = useCallback(async (exclusionCriteria: string[]) => {
    setStep("running");
    setError(null);
    setProgress(null);
    setResult(null);
    setIsPaused(false);

    try {
      unlistenRef.current = await listen<AiScreeningProgress>("screen:progress", (e) => {
        setProgress(e.payload);
      });
      const res = await api.runAiScreening(exclusionCriteria);
      setResult(res);
      setStep("done");
    } catch (e: any) {
      const msg: string = e?.message ?? String(e);
      setError(
        msg.includes("chat model")
          ? "Pick a chat model in Settings first."
          : msg.includes("Connection refused") || msg.includes("error sending request")
          ? "Could not reach LM Studio — check Settings."
          : msg,
      );
      setStep("running"); // stay on running step to show error inline
    } finally {
      unlistenRef.current?.();
      unlistenRef.current = null;
    }
  }, []);

  const pause = useCallback(async () => {
    await api.pauseAiScreening();
    setIsPaused(true);
  }, []);

  const resume = useCallback(async () => {
    await api.resumeAiScreening();
    setIsPaused(false);
  }, []);

  const cancel = useCallback(async () => {
    await api.cancelAiScreening();
    setIsPaused(false);
  }, []);

  const reset = useCallback(() => {
    setStep("idle");
    setProgress(null);
    setResult(null);
    setError(null);
    setIsPaused(false);
  }, []);

  return { step, progress, result, error, isPaused, start, pause, resume, cancel, reset };
}

export function ScreeningPanel({ project, refreshKey, onChanged, onOpenAiModal }: Props) {
  const [activeTab, setActiveTab] = useState<ScreeningTab>("pending");
  const [articles, setArticles] = useState<ArticleRow[]>([]);
  const [total, setTotal] = useState(0);
  const [tabCounts, setTabCounts] = useState<Record<ScreeningTab, number>>({
    pending: 0,
    included: 0,
    excluded: 0,
  });
  const [criteria, setCriteria] = useState<ExclusionCriterionRow[]>([]);
  const [journals, setJournals] = useState<JournalRow[]>([]);
  const [offset, setOffset] = useState(0);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [loading, setLoading] = useState(true);

  const jq = parseJournalQuality(project.journal_quality);

  const reload = async (tab = activeTab, off = offset) => {
    setLoading(true);
    const [arts, cntPending, cntIncluded, cntExcluded, crits, jList] = await Promise.all([
      api.listScreeningTab(tab, off, BATCH_SIZE),
      api.countScreeningTab("pending"),
      api.countScreeningTab("included"),
      api.countScreeningTab("excluded"),
      api.listCriteria(),
      api.listJournals(),
    ]);
    setArticles(arts);
    setTabCounts({ pending: cntPending, included: cntIncluded, excluded: cntExcluded });
    setTotal(tab === "pending" ? cntPending : tab === "included" ? cntIncluded : cntExcluded);
    setCriteria(crits.filter((c) => c.number > 1));
    setJournals(jList);
    setLoading(false);
  };

  useEffect(() => {
    setOffset(0);
    let cancelled = false;
    setLoading(true);
    Promise.all([
      api.listScreeningTab(activeTab, 0, BATCH_SIZE),
      api.countScreeningTab("pending"),
      api.countScreeningTab("included"),
      api.countScreeningTab("excluded"),
      api.listCriteria(),
      api.listJournals(),
    ]).then(([arts, cntPending, cntIncluded, cntExcluded, crits, jList]) => {
      if (cancelled) return;
      setArticles(arts);
      setTabCounts({ pending: cntPending, included: cntIncluded, excluded: cntExcluded });
      setTotal(activeTab === "pending" ? cntPending : activeTab === "included" ? cntIncluded : cntExcluded);
      setCriteria(crits.filter((c) => c.number > 1));
      setJournals(jList);
      setLoading(false);
    }).catch(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [refreshKey, activeTab]);

  const journalMap = useMemo(() => {
    const m = new Map<string, JournalRow>();
    for (const j of journals) m.set(normalizeJournal(j.name), j);
    return m;
  }, [journals]);

  const lookupJournal = (articleJournal: string | null | undefined): JournalRow | null => {
    if (!articleJournal) return null;
    return journalMap.get(normalizeJournal(articleJournal)) ?? null;
  };

  const onDecision = async () => {
    onChanged();
    await reload();
  };

  const totalPages = Math.max(1, Math.ceil(total / BATCH_SIZE));
  const currentPage = Math.floor(offset / BATCH_SIZE) + 1;

  return (
    <div className="pl-6 pr-4 xl:pl-12 xl:pr-8">
      {/* Header row */}
      <div className="flex items-end justify-between mb-5">
        <div>
          <div className="eyebrow text-[var(--color-paper-500)]">Title & abstract screening</div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">
            Decide one at a time
          </h2>
        </div>
        {activeTab === "pending" && (
          <button
            type="button"
            onClick={onOpenAiModal}
            className="no-drag inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-accent-600)] border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)] transition-paper"
          >
            <Sparkles size={12} strokeWidth={2} />
            Screen with AI
          </button>
        )}
      </div>

      {/* Tabs */}
      <div className="flex items-center gap-1 mb-5 border-b border-[var(--color-paper-200)]">
        {TABS.map((t) => {
          const count = tabCounts[t.id];
          const isActive = activeTab === t.id;
          return (
            <button
              key={t.id}
              type="button"
              onClick={() => { setActiveTab(t.id); setOffset(0); }}
              className={cn(
                "inline-flex items-center gap-2 px-4 py-2 text-[13px] font-medium transition-paper -mb-px border-b-2",
                isActive
                  ? "border-[var(--color-ink-800)] text-[var(--color-ink-800)]"
                  : "border-transparent text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)]",
              )}
            >
              {t.label}
              <span
                className={cn(
                  "inline-flex items-center justify-center min-w-[22px] h-[18px] px-1.5 rounded-full",
                  "text-[11px] font-mono font-semibold tabular-nums",
                  isActive
                    ? "bg-[var(--color-ink-800)] text-white"
                    : "bg-[var(--color-paper-100)] text-[var(--color-paper-600)] border border-[var(--color-paper-300)]",
                )}
              >
                {count}
              </span>
            </button>
          );
        })}
      </div>

      {/* Description + Pagination */}
      <div className="flex items-center justify-between mb-4">
        <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed max-w-xl">
          {activeTab === "pending"
            ? "Keep each article or apply an exclusion criterion. AI-flagged articles show the AI rationale."
            : activeTab === "included"
            ? "Articles kept after screening. Click Revert to return an article to Pending."
            : "Articles excluded from the corpus. Click Revert to return an article to Pending."}
        </p>
        <div className="flex items-center gap-1 shrink-0">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setOffset(Math.max(0, offset - BATCH_SIZE))}
            disabled={offset === 0}
          >
            <ArrowLeft size={13} strokeWidth={2} />
            Prev
          </Button>
          <span className="px-3 text-[12px] font-semibold tabular-nums text-[var(--color-ink-700)]">
            {currentPage} / {totalPages}
          </span>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setOffset(offset + BATCH_SIZE)}
            disabled={offset + BATCH_SIZE >= total}
          >
            Next
            <ArrowRight size={13} strokeWidth={2} />
          </Button>
        </div>
      </div>

      {loading && (
        <div className="flex items-center gap-2 text-[13px] text-[var(--color-paper-500)]">
          <Loader2 size={14} className="animate-spin" /> Loading…
        </div>
      )}

      {!loading && articles.length === 0 && (
        <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-12 text-center">
          <Filter className="mx-auto mb-3 text-[var(--color-paper-400)]" size={24} strokeWidth={1.5} />
          <p className="text-[14px] text-[var(--color-paper-700)] font-medium mb-1">
            {activeTab === "pending" ? "Nothing to screen" : activeTab === "included" ? "No articles included yet" : "No articles excluded yet"}
          </p>
          <p className="text-[13px] text-[var(--color-paper-500)] max-w-md mx-auto leading-relaxed">
            {activeTab === "pending"
              ? "Import records in Stage 1 or all articles have already been screened."
              : activeTab === "included"
              ? "Include articles from the Pending tab to see them here."
              : "Exclude articles from the Pending tab to see them here."}
          </p>
        </div>
      )}

      <div className="space-y-3">
        {articles.map((a) => (
          <ScreeningCard
            key={a.id}
            article={a}
            criteria={criteria}
            journalMatch={lookupJournal(a.journal)}
            jq={jq}
            expanded={expanded.has(a.id)}
            onToggle={() => {
              setExpanded((s) => {
                const n = new Set(s);
                if (n.has(a.id)) n.delete(a.id);
                else n.add(a.id);
                return n;
              });
            }}
            onDecision={onDecision}
          />
        ))}
      </div>

      {!loading && articles.some((a) => lookupJournal(a.journal)?.impact_factor != null) && (
        <p className="mt-6 text-[11px] text-[var(--color-paper-400)] leading-relaxed max-w-2xl">
          * <strong className="font-medium text-[var(--color-paper-500)]">IF</strong> — Impact
          factor sourced from your journal scope list. Use as a screening guide only.
        </p>
      )}

    </div>
  );
}

function ScreeningCard({
  article,
  criteria,
  journalMatch,
  jq,
  expanded,
  onToggle,
  onDecision,
}: {
  article: ArticleRow;
  criteria: ExclusionCriterionRow[];
  journalMatch: JournalRow | null;
  jq: { abs_min?: string; abdc_min?: string; if_min?: number };
  expanded: boolean;
  onToggle: () => void;
  onDecision: () => Promise<void>;
}) {
  const [busy, setBusy] = useState(false);
  const [rationaleOpen, setRationaleOpen] = useState(false);
  // Only show "Included" for articles the user explicitly kept via screening.
  // Articles auto-included by PDF attachment have screened_at = null.
  const isDecided = article.status === "included" && article.screened_at !== null;

  // Determine journal quality status
  const absFails = jq.abs_min && !meetsAbsThreshold(journalMatch?.abs_rating, jq.abs_min);
  const abdcFails = jq.abdc_min && !meetsAbdcThreshold(journalMatch?.abdc_rating, jq.abdc_min);
  const ifFails = jq.if_min != null && !meetsIfThreshold(journalMatch?.impact_factor, jq.if_min);
  const qualityFlagged = absFails || abdcFails || ifFails;

  const keep = async () => {
    setBusy(true);
    try {
      await api.keepArticle(article.id);
      await onDecision();
    } finally {
      setBusy(false);
    }
  };

  const exclude = async (criterionNumber: number) => {
    setBusy(true);
    try {
      await api.excludeArticle(article.id, criterionNumber);
      await onDecision();
    } finally {
      setBusy(false);
    }
  };

  const revert = async () => {
    setBusy(true);
    try {
      await api.unscreenArticle(article.id);
      await onDecision();
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card
      className={cn(
        "p-5",
        isDecided && "bg-[var(--color-ok-500)]/5 border-[var(--color-ok-500)]/30",
        qualityFlagged && !isDecided && article.status !== "included" && "border-[var(--color-warn-500)]/40",
      )}
    >
      <div className="flex items-start gap-4">
        <div className="font-mono text-[10px] text-[var(--color-paper-500)] tabular-nums w-8 text-right shrink-0 mt-1">
          #{article.id}
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="font-serif text-[16px] leading-snug text-[var(--color-ink-900)]">
            {article.title}
          </h3>
          <div className="mt-1 flex items-baseline gap-3 text-[12px] text-[var(--color-paper-600)]">
            {article.authors && <span className="line-clamp-1">{article.authors}</span>}
            {article.year && <span className="font-mono tabular-nums">{article.year}</span>}
          </div>

          {/* Journal line with quality badges */}
          {article.journal && (
            <div className="mt-1 flex items-center gap-2 flex-wrap">
              <span className="text-[11px] italic text-[var(--color-paper-500)]">
                {article.journal}
              </span>
              {journalMatch ? (
                <div className="flex items-center gap-1">
                  {journalMatch.abs_rating && (
                    <JournalBadge label="ABS" value={journalMatch.abs_rating} flagged={!!absFails} />
                  )}
                  {journalMatch.abdc_rating && (
                    <JournalBadge label="ABDC" value={journalMatch.abdc_rating} flagged={!!abdcFails} />
                  )}
                  {journalMatch.impact_factor != null && (
                    <JournalBadge
                      label="IF"
                      value={journalMatch.impact_factor.toFixed(2)}
                      flagged={!!ifFails}
                      title="Impact factor from the journal scope list"
                    />
                  )}
                </div>
              ) : (
                (jq.abs_min || jq.abdc_min || jq.if_min != null) && (
                  <span className="text-[10px] text-[var(--color-paper-400)] italic">
                    not in journal list
                  </span>
                )
              )}
            </div>
          )}

          {/* Quality warning */}
          {qualityFlagged && !isDecided && (
            <div className="mt-2 flex items-center gap-1.5 text-[11px] text-[var(--color-warn-600)]">
              <AlertTriangle size={11} strokeWidth={1.75} />
              <span>
                Below quality threshold —{" "}
                {[
                  absFails && `ABS min ${jq.abs_min}`,
                  abdcFails && `ABDC min ${jq.abdc_min}`,
                  ifFails && `IF min ${jq.if_min}`,
                ]
                  .filter(Boolean)
                  .join(", ")}
              </span>
            </div>
          )}

          {/* AI rationale — shown for AI-screened and AI-uncertain articles */}
          {article.ai_rationale && (
            <>
              <button
                onClick={() => setRationaleOpen(true)}
                className="mt-2 flex items-start gap-1.5 text-[11px] text-left w-full group"
              >
                <Bot size={11} strokeWidth={1.75} className="shrink-0 mt-0.5 text-[var(--color-accent-500)]" />
                <span className={cn(
                  "leading-snug group-hover:underline",
                  article.ai_confidence !== null && article.status === "retrieved"
                    ? "text-[var(--color-paper-500)] italic"
                    : "text-[var(--color-paper-500)]",
                )}>
                  {article.ai_confidence !== null && article.status === "retrieved" && (
                    <span className="font-medium text-[var(--color-warn-600)] not-italic mr-1">
                      AI couldn't decide —
                    </span>
                  )}
                  <span className="line-clamp-2">{article.ai_rationale}</span>
                  {article.ai_confidence !== null && (
                    <span className="ml-1 text-[var(--color-paper-400)] not-italic">
                      ({Math.round(article.ai_confidence * 100)}% confidence)
                    </span>
                  )}
                </span>
              </button>
              <AnimatePresence>
                {rationaleOpen && (
                  <motion.div
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.12 }}
                    className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/30 backdrop-blur-sm"
                    onClick={() => setRationaleOpen(false)}
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
                        <Bot size={14} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                        <span className="text-[13px] font-semibold text-[var(--color-ink-900)] flex-1">
                          AI screener — rationale
                        </span>
                        <button
                          onClick={() => setRationaleOpen(false)}
                          className="p-1 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] text-[var(--color-paper-500)] transition-paper"
                        >
                          <X size={13} strokeWidth={2} />
                        </button>
                      </div>
                      <div className="px-5 py-4">
                        <p className="text-[13px] text-[var(--color-ink-800)] leading-relaxed">
                          {article.ai_rationale}
                          {article.ai_confidence !== null && (
                            <span className="ml-2 text-[11px] text-[var(--color-paper-400)] font-mono">
                              ({Math.round(article.ai_confidence * 100)}% confidence)
                            </span>
                          )}
                        </p>
                      </div>
                    </motion.div>
                  </motion.div>
                )}
              </AnimatePresence>
            </>
          )}

          <button
            onClick={onToggle}
            className="mt-2 text-[11px] text-[var(--color-ink-600)] hover:text-[var(--color-ink-900)] font-medium inline-flex items-center gap-1"
          >
            {expanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
            {expanded ? "Hide abstract" : "Show abstract"}
          </button>
          <AnimatePresence>
            {expanded && article.abstract_text && (
              <motion.p
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: "auto" }}
                exit={{ opacity: 0, height: 0 }}
                className="prose-editorial text-[13px] mt-2 leading-[1.7] text-[var(--color-paper-700)] overflow-hidden"
              >
                {article.abstract_text}
              </motion.p>
            )}
          </AnimatePresence>
        </div>
      </div>

      <div className="mt-4 pt-4 border-t border-[var(--color-paper-200)] flex items-center gap-2 flex-wrap">
        {isDecided ? (
          <>
            <span className="inline-flex items-center gap-1.5 text-[12px] font-medium text-[var(--color-ok-500)]">
              <Check size={13} strokeWidth={2.25} />
              Included
            </span>
            <Button onClick={revert} variant="ghost" size="sm" disabled={busy}>
              <RotateCcw size={12} strokeWidth={1.75} />
              Revert
            </Button>
          </>
        ) : article.status === "excluded" ? (
          <>
            <span className="inline-flex items-center gap-1.5 text-[12px] font-medium text-[var(--color-warn-600)]">
              Excluded
            </span>
            <Button onClick={revert} variant="ghost" size="sm" disabled={busy}>
              <RotateCcw size={12} strokeWidth={1.75} />
              Revert
            </Button>
          </>
        ) : (
          <>
            <Button onClick={keep} variant="primary" size="sm" disabled={busy}>
              <Check size={13} strokeWidth={2} />
              Include
            </Button>
            <span className="text-[11px] text-[var(--color-paper-500)] mx-2">or exclude with…</span>
            <div className="flex flex-wrap gap-1.5">
              {criteria.map((c) => (
                <button
                  key={c.id}
                  onClick={() => exclude(c.number)}
                  disabled={busy}
                  className="px-2.5 py-1 text-[11px] rounded-full border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] hover:bg-[var(--color-error-500)]/5 hover:border-[var(--color-error-500)]/40 hover:text-[var(--color-error-500)] text-[var(--color-paper-700)] transition-paper"
                  title={c.rationale ?? c.label}
                >
                  #{c.number} {c.label}
                </button>
              ))}
            </div>
          </>
        )}
      </div>
    </Card>
  );
}

function JournalBadge({
  label,
  value,
  flagged,
  title,
}: {
  label: string;
  value: string;
  flagged: boolean;
  title?: string;
}) {
  return (
    <span
      title={title}
      className={cn(
        "inline-flex items-center gap-1 px-1.5 py-px rounded text-[10px] font-semibold font-mono cursor-default",
        flagged
          ? "bg-[var(--color-warn-500)]/15 text-[var(--color-warn-600)] border border-[var(--color-warn-500)]/30"
          : "bg-[var(--color-paper-100)] text-[var(--color-paper-600)] border border-[var(--color-paper-300)]",
      )}
    >
      {label} {value}
    </span>
  );
}
