import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  ChevronDown,
  ChevronUp,
  Filter,
  Loader2,
  RotateCcw,
} from "lucide-react";
import {
  api,
  type ArticleRow,
  type ExclusionCriterionRow,
} from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { cn } from "../../lib/utils";

interface Props {
  refreshKey: number;
  onChanged: () => void;
}

const BATCH_SIZE = 20;

export function ScreeningPanel({ refreshKey, onChanged }: Props) {
  const [articles, setArticles] = useState<ArticleRow[]>([]);
  const [criteria, setCriteria] = useState<ExclusionCriterionRow[]>([]);
  const [offset, setOffset] = useState(0);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [loading, setLoading] = useState(true);

  const reload = async (off = offset) => {
    setLoading(true);
    const [arts, crits] = await Promise.all([
      api.listScreeningBatch(off, BATCH_SIZE),
      api.listCriteria(),
    ]);
    setArticles(arts);
    setCriteria(crits.filter((c) => c.number > 1)); // exclude duplicate criterion (#1)
    setLoading(false);
  };

  useEffect(() => {
    reload();
  }, [refreshKey, offset]);

  const onDecision = async () => {
    onChanged();
    await reload();
  };

  return (
    <div className="max-w-5xl">
      <div className="flex items-end justify-between mb-6">
        <div>
          <div className="eyebrow text-[var(--color-paper-500)]">Title & abstract screening</div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">
            Decide one at a time
          </h2>
          <p className="text-[13px] text-[var(--color-paper-600)] mt-1.5 max-w-xl leading-relaxed">
            For each article: keep it (and it joins the included pool), or apply an exclusion
            criterion. Already-decided articles show their decision; click <RotateCcw size={11} className="inline -mt-0.5" /> to revert.
          </p>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setOffset(Math.max(0, offset - BATCH_SIZE))}
            disabled={offset === 0}
          >
            <ArrowLeft size={13} strokeWidth={2} />
            Prev
          </Button>
          <span className="px-3 text-[11px] font-mono tabular-nums text-[var(--color-paper-500)]">
            Batch {Math.floor(offset / BATCH_SIZE) + 1}
          </span>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setOffset(offset + BATCH_SIZE)}
            disabled={articles.length < BATCH_SIZE}
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
            No articles to screen
          </p>
          <p className="text-[13px] text-[var(--color-paper-500)] max-w-md mx-auto leading-relaxed">
            Either you haven't imported records yet, or you've screened the entire pool.
          </p>
        </div>
      )}

      <div className="space-y-3">
        {articles.map((a) => (
          <ScreeningCard
            key={a.id}
            article={a}
            criteria={criteria}
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
    </div>
  );
}

function ScreeningCard({
  article,
  criteria,
  expanded,
  onToggle,
  onDecision,
}: {
  article: ArticleRow;
  criteria: ExclusionCriterionRow[];
  expanded: boolean;
  onToggle: () => void;
  onDecision: () => Promise<void>;
}) {
  const [busy, setBusy] = useState(false);
  const isIncluded = article.status === "included";
  const isDecided = isIncluded;

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
          {article.journal && (
            <div className="text-[11px] italic text-[var(--color-paper-500)] line-clamp-1 mt-0.5">
              {article.journal}
            </div>
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
