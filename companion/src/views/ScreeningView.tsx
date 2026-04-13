import { useCallback, useEffect, useRef, useState } from "react";
import { ArrowLeft, Check, ChevronRight, SkipForward, X } from "lucide-react";
import {
  listScreening,
  listCriteria,
  keepArticle,
  excludeArticle,
  unscreenArticle,
  type ArticleRow,
  type CriterionRow,
} from "../lib/api";

interface Props {
  onBack: () => void;
}

export function ScreeningView({ onBack }: Props) {
  const [articles, setArticles] = useState<ArticleRow[]>([]);
  const [criteria, setCriteria] = useState<CriterionRow[]>([]);
  const [index, setIndex] = useState(0);
  const [loading, setLoading] = useState(true);
  const [acting, setActing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showCriteria, setShowCriteria] = useState(false);
  const [lastAction, setLastAction] = useState<"keep" | "exclude" | "skip" | null>(null);
  const offset = useRef(0);
  const exhausted = useRef(false);

  const loadMore = useCallback(async () => {
    if (exhausted.current) return;
    const batch = await listScreening({ offset: offset.current, limit: 20 });
    if (batch.length < 20) exhausted.current = true;
    offset.current += batch.length;
    setArticles((prev) => [...prev, ...batch]);
  }, []);

  useEffect(() => {
    Promise.all([loadMore(), listCriteria()])
      .then(([, c]) => setCriteria(c))
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, [loadMore]);

  const current = articles[index];

  function advance(action: "keep" | "exclude" | "skip") {
    setLastAction(action);
    setShowCriteria(false);
    setIndex((i) => i + 1);
    if (!exhausted.current && index + 2 >= articles.length - 3) loadMore();
  }

  async function handleKeep() {
    if (!current || acting) return;
    setActing(true);
    try {
      await keepArticle(current.id);
      advance("keep");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed");
    } finally {
      setActing(false);
    }
  }

  async function handleExclude(criterionNumber: number) {
    if (!current || acting) return;
    setActing(true);
    try {
      await excludeArticle(current.id, criterionNumber);
      advance("exclude");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed");
    } finally {
      setActing(false);
    }
  }

  function handleSkip() {
    if (!current || acting) return;
    advance("skip");
  }

  async function handleUndo() {
    if (index === 0 || acting || lastAction === "skip") return;
    const prev = articles[index - 1];
    setActing(true);
    try {
      await unscreenArticle(prev.id);
      setIndex((i) => i - 1);
      setLastAction(null);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed");
    } finally {
      setActing(false);
    }
  }

  const done = index >= articles.length && exhausted.current;
  const progress = articles.length > 0 ? Math.min(index / (exhausted.current ? articles.length : articles.length + 1), 1) : 0;

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center bg-[#f0ede6]">
        <div className="w-5 h-5 border-2 border-[var(--color-ink-200)] border-t-[var(--color-ink-600)] rounded-full animate-spin" />
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-[#f0ede6]">
      {/* Top nav */}
      <div className="flex items-center gap-3 px-4 pt-4 pb-3 safe-top">
        <button
          onClick={onBack}
          className="w-9 h-9 flex items-center justify-center rounded-full bg-white shadow-sm"
        >
          <ArrowLeft size={18} strokeWidth={2} className="text-[var(--color-ink-700)]" />
        </button>
        <span className="font-semibold text-[var(--color-ink-800)]">Screen articles</span>
        {articles.length > 0 && !done && (
          <span className="ml-auto text-sm text-[var(--color-paper-400)] tabular-nums">
            {index + 1} / {exhausted.current ? articles.length : `${articles.length}+`}
          </span>
        )}
      </div>

      {/* Progress bar */}
      {articles.length > 0 && (
        <div className="h-0.5 bg-[var(--color-paper-200)] mx-4 rounded-full overflow-hidden">
          <div
            className="h-full bg-[var(--color-ink-600)] rounded-full transition-all duration-300"
            style={{ width: `${progress * 100}%` }}
          />
        </div>
      )}

      {/* Error banner */}
      {error && (
        <div className="mx-4 mt-3 px-4 py-3 rounded-[var(--radius-md)] bg-[#fef2f2] border border-[#fecaca]">
          <p className="text-sm text-[var(--color-error-500)]">{error}</p>
          <button onClick={() => setError(null)} className="text-xs text-[var(--color-error-500)] underline mt-1">Dismiss</button>
        </div>
      )}

      {done ? (
        /* All done state */
        <div className="flex-1 flex flex-col items-center justify-center px-8 gap-4">
          <div className="w-16 h-16 rounded-full bg-[var(--color-ok-500)] flex items-center justify-center shadow-lg">
            <Check size={28} strokeWidth={2.5} className="text-white" />
          </div>
          <div className="text-center">
            <p className="text-xl font-bold text-[var(--color-ink-800)]">All caught up</p>
            <p className="text-sm text-[var(--color-paper-500)] mt-1">No more articles to screen.</p>
          </div>
          <button
            onClick={onBack}
            className="mt-2 px-6 py-3 rounded-[var(--radius-pill)] bg-[var(--color-ink-700)] text-white font-medium text-sm"
          >
            Back to home
          </button>
        </div>
      ) : !current ? (
        <div className="flex-1 flex items-center justify-center">
          <p className="text-sm text-[var(--color-paper-400)]">No articles to screen.</p>
        </div>
      ) : (
        <div className="flex-1 flex flex-col overflow-hidden">
          {/* Article card — scrollable content */}
          <div className="flex-1 overflow-y-auto px-4 py-4">
            <div
              className="bg-white rounded-[var(--radius-xl)] p-5 space-y-3"
              style={{ boxShadow: "0 2px 8px rgba(0,0,0,0.06), 0 1px 3px rgba(0,0,0,0.04)" }}
            >
              {(current.year || current.journal) && (
                <div className="flex items-center gap-2 flex-wrap">
                  {current.year && (
                    <span className="text-xs font-semibold text-[var(--color-paper-500)] bg-[var(--color-paper-100)] px-2 py-0.5 rounded-[var(--radius-sm)]">
                      {current.year}
                    </span>
                  )}
                  {current.journal && (
                    <span className="text-xs text-[var(--color-paper-400)] truncate">{current.journal}</span>
                  )}
                </div>
              )}

              <h2 className="text-lg font-bold text-[var(--color-ink-800)] leading-snug">
                {current.title}
              </h2>

              {current.authors && (
                <p className="text-sm text-[var(--color-paper-500)]">{current.authors}</p>
              )}

              {current.abstract_text && (
                <p className="text-sm text-[var(--color-ink-700)] leading-relaxed pt-1 border-t border-[var(--color-paper-100)]">
                  {current.abstract_text}
                </p>
              )}
            </div>
          </div>

          {/* Action sheet */}
          <div className="px-4 pb-6 pt-3 space-y-3 bg-[#f0ede6]">
            {/* Include button */}
            <button
              onClick={handleKeep}
              disabled={acting}
              className="w-full flex items-center justify-center gap-2.5 py-4 rounded-[var(--radius-xl)] bg-[#2a5c45] text-white font-semibold text-base shadow-sm active:scale-[0.98] transition-transform disabled:opacity-50"
            >
              <Check size={20} strokeWidth={2.5} />
              Include
            </button>

            {/* Exclude / Skip row */}
            <div className="grid grid-cols-2 gap-3">
              <button
                onClick={() => setShowCriteria((v) => !v)}
                disabled={acting}
                className={`flex items-center justify-center gap-2 py-3.5 rounded-[var(--radius-xl)] font-semibold text-sm border-2 transition-all active:scale-[0.98] disabled:opacity-50 ${showCriteria ? "bg-[#5c2a2a] text-white border-[#5c2a2a]" : "bg-white text-[#a64141] border-[#fecaca]"}`}
              >
                <X size={17} strokeWidth={2.5} />
                Exclude
              </button>
              <button
                onClick={handleSkip}
                disabled={acting}
                className="flex items-center justify-center gap-2 py-3.5 rounded-[var(--radius-xl)] bg-white font-semibold text-sm text-[var(--color-paper-500)] border-2 border-[var(--color-paper-200)] active:scale-[0.98] transition-transform disabled:opacity-50"
              >
                <SkipForward size={17} strokeWidth={2} />
                Skip
              </button>
            </div>

            {/* Criteria picker */}
            {showCriteria && criteria.length > 0 && (
              <div className="space-y-2">
                <p className="text-xs font-semibold text-[var(--color-paper-500)] uppercase tracking-wider px-1">
                  Select exclusion criterion
                </p>
                <div className="space-y-2">
                  {criteria.map((c) => (
                    <button
                      key={c.id}
                      onClick={() => handleExclude(c.number)}
                      disabled={acting}
                      className="w-full flex items-center gap-3 px-4 py-3 rounded-[var(--radius-lg)] bg-white border border-[#fecaca] text-left active:bg-[#fef2f2] transition-colors disabled:opacity-50"
                    >
                      <div className="w-6 h-6 rounded-full bg-[#fef2f2] border border-[#fecaca] flex items-center justify-center shrink-0">
                        <X size={12} strokeWidth={2.5} className="text-[var(--color-error-500)]" />
                      </div>
                      <span className="text-sm text-[var(--color-ink-700)] font-medium">{c.label}</span>
                      <ChevronRight size={14} strokeWidth={2} className="text-[var(--color-paper-300)] ml-auto shrink-0" />
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Undo */}
            {index > 0 && lastAction !== "skip" && (
              <button
                onClick={handleUndo}
                disabled={acting}
                className="w-full py-2 text-sm text-[var(--color-paper-400)] disabled:opacity-50"
              >
                Undo last decision
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
