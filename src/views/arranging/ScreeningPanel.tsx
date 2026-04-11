import { useEffect, useMemo, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  AlertTriangle,
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
  type JournalRow,
  type ProjectRow,
} from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { cn } from "../../lib/utils";

interface Props {
  project: ProjectRow;
  refreshKey: number;
  onChanged: () => void;
}

const BATCH_SIZE = 20;

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

export function ScreeningPanel({ project, refreshKey, onChanged }: Props) {
  const [articles, setArticles] = useState<ArticleRow[]>([]);
  const [criteria, setCriteria] = useState<ExclusionCriterionRow[]>([]);
  const [journals, setJournals] = useState<JournalRow[]>([]);
  const [offset, setOffset] = useState(0);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [loading, setLoading] = useState(true);

  const jq = parseJournalQuality(project.journal_quality);

  const reload = async (off = offset) => {
    setLoading(true);
    const [arts, crits, jList] = await Promise.all([
      api.listScreeningBatch(off, BATCH_SIZE),
      api.listCriteria(),
      api.listJournals(),
    ]);
    setArticles(arts);
    setCriteria(crits.filter((c) => c.number > 1));
    setJournals(jList);
    setLoading(false);
  };

  useEffect(() => {
    reload();
  }, [refreshKey, offset]);

  // Build a lookup map: normalised journal name → JournalRow
  const journalMap = useMemo(() => {
    const m = new Map<string, JournalRow>();
    for (const j of journals) {
      m.set(normalizeJournal(j.name), j);
    }
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
            criterion. Already-decided articles show their decision; click{" "}
            <RotateCcw size={11} className="inline -mt-0.5" /> to revert.
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

      {/* Footnote — shown when at least one article in this batch has an IF badge */}
      {!loading && articles.some((a) => lookupJournal(a.journal)?.impact_factor != null) && (
        <p className="mt-6 text-[11px] text-[var(--color-paper-400)] leading-relaxed max-w-2xl">
          * <strong className="font-medium text-[var(--color-paper-500)]">IF</strong> — Impact
          factor sourced from your journal scope list. Values reflect the data entered when the
          journal list was imported and may not match the current year's official JCR release.
          Use as a screening guide only.
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
  const isIncluded = article.status === "included";
  const isDecided = isIncluded;

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
        qualityFlagged && !isDecided && "border-[var(--color-warn-500)]/40",
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
