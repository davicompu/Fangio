import { useEffect, useMemo, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  ChevronDown,
  ChevronUp,
  Loader2,
  Plus,
  Sparkles,
  Tag,
  X,
} from "lucide-react";
import {
  api,
  type ArticleRow,
  type CodingRow,
  type DimensionInfo,
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

const BATCH_SIZE = 20;

export function CodingPanel({ project, refreshKey, onChanged }: Props) {
  const [dimensions, setDimensions] = useState<DimensionInfo[]>([]);
  const [articles, setArticles] = useState<ArticleRow[]>([]);
  const [codings, setCodings] = useState<CodingRow[]>([]);
  const [offset, setOffset] = useState(0);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [loading, setLoading] = useState(true);

  const reload = async (off = offset) => {
    setLoading(true);
    const [dims, batch] = await Promise.all([
      api.listFrameworkDimensions(),
      api.listCodingBatch(off, BATCH_SIZE),
    ]);
    setDimensions(dims);
    setArticles(batch.articles);
    setCodings(batch.codings);
    setLoading(false);
  };

  useEffect(() => {
    reload();
  }, [refreshKey, offset]);

  const codingsByArticle = useMemo(() => {
    const m = new Map<number, CodingRow[]>();
    for (const c of codings) {
      if (!m.has(c.article_id)) m.set(c.article_id, []);
      m.get(c.article_id)!.push(c);
    }
    return m;
  }, [codings]);

  const onCodingChanged = async () => {
    onChanged();
    await reload();
  };

  const toggleExpand = (id: number) => {
    setExpanded((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  if (!project.framework) {
    return <div className="text-[13px] text-[var(--color-paper-500)]">No framework selected.</div>;
  }

  return (
    <div className="max-w-6xl">
      <div className="flex items-end justify-between mb-6">
        <div>
          <div className="eyebrow text-[var(--color-paper-500)]">Batch coding</div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">
            {project.framework} · {dimensions.length} dimensions
          </h2>
          <p className="text-[13px] text-[var(--color-paper-600)] mt-1.5">
            Articles {offset + 1}–{offset + articles.length}. Code one at a time, expand for the
            full abstract, or click <strong>Suggest</strong> on any dimension for local-model
            help.
          </p>
        </div>
        <BatchPager offset={offset} batchSize={BATCH_SIZE} count={articles.length} setOffset={setOffset} />
      </div>

      {loading && (
        <div className="flex items-center gap-2 text-[13px] text-[var(--color-paper-500)]">
          <Loader2 size={14} className="animate-spin" /> Loading batch…
        </div>
      )}

      {!loading && articles.length === 0 && (
        <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-12 text-center">
          <p className="text-[14px] text-[var(--color-paper-700)] font-medium mb-1">
            No articles to code
          </p>
          <p className="text-[13px] text-[var(--color-paper-500)] max-w-md mx-auto leading-relaxed">
            Either you haven't imported any records yet (return to Stage 1) or every article has
            been screened. Check the Library or PRISMA tab.
          </p>
        </div>
      )}

      <div className="space-y-4">
        {articles.map((article, i) => (
          <ArticleCodingCard
            key={article.id}
            index={offset + i}
            article={article}
            dimensions={dimensions}
            codings={codingsByArticle.get(article.id) ?? []}
            expanded={expanded.has(article.id)}
            onToggleExpand={() => toggleExpand(article.id)}
            onChanged={onCodingChanged}
          />
        ))}
      </div>
    </div>
  );
}

function BatchPager({
  offset,
  batchSize,
  count,
  setOffset,
}: {
  offset: number;
  batchSize: number;
  count: number;
  setOffset: (n: number) => void;
}) {
  return (
    <div className="flex items-center gap-1">
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setOffset(Math.max(0, offset - batchSize))}
        disabled={offset === 0}
      >
        <ArrowLeft size={13} strokeWidth={2} />
        Prev
      </Button>
      <span className="px-3 text-[11px] font-mono tabular-nums text-[var(--color-paper-500)]">
        Batch {Math.floor(offset / batchSize) + 1}
      </span>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setOffset(offset + batchSize)}
        disabled={count < batchSize}
      >
        Next
        <ArrowRight size={13} strokeWidth={2} />
      </Button>
    </div>
  );
}

function ArticleCodingCard({
  index,
  article,
  dimensions,
  codings,
  expanded,
  onToggleExpand,
  onChanged,
}: {
  index: number;
  article: ArticleRow;
  dimensions: DimensionInfo[];
  codings: CodingRow[];
  expanded: boolean;
  onToggleExpand: () => void;
  onChanged: () => Promise<void>;
}) {
  return (
    <Card className="overflow-hidden">
      <div className="px-5 py-4 flex items-start gap-4">
        <div className="font-mono text-[10px] text-[var(--color-paper-500)] tabular-nums w-8 text-right shrink-0 mt-1">
          {String(index + 1).padStart(2, "0")}
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
            onClick={onToggleExpand}
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

      <div className="border-t border-[var(--color-paper-200)] divide-y divide-[var(--color-paper-200)]">
        {dimensions.map((dim) => (
          <DimensionRow
            key={dim.name}
            article={article}
            dimension={dim}
            codings={codings.filter((c) => c.dimension === dim.name)}
            onChanged={onChanged}
          />
        ))}
      </div>
    </Card>
  );
}

function DimensionRow({
  article,
  dimension,
  codings,
  onChanged,
}: {
  article: ArticleRow;
  dimension: DimensionInfo;
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
    // Close suggestions immediately so a second click can't fire the same save
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
    <div className="px-5 py-3.5 grid grid-cols-[140px_1fr_auto] gap-4 items-start">
      <div className="pt-1">
        <div className="text-[12px] font-semibold tracking-tight text-[var(--color-ink-800)]">
          {dimension.name}
        </div>
        <div className="text-[10px] text-[var(--color-paper-500)] mt-0.5 line-clamp-2 leading-relaxed">
          {dimension.description}
        </div>
      </div>

      <div className="min-w-0">
        <div className="flex flex-wrap gap-1.5 items-center">
          {codings.map((c) => (
            <AppliedCode key={c.id} coding={c} onRemove={() => removeCode(c.id)} />
          ))}
          {codings.length === 0 && !adding && (
            <span className="text-[11px] italic text-[var(--color-paper-400)]">no codes</span>
          )}
          {adding && (
            <input
              autoFocus
              value={newCode}
              onChange={(e) => setNewCode(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") addCode(newCode, "researcher");
                if (e.key === "Escape") {
                  setAdding(false);
                  setNewCode("");
                }
              }}
              placeholder="Type a code…"
              className="px-2 py-0.5 text-[12px] rounded-full bg-white border border-[var(--color-ink-700)] focus:outline-none w-32"
            />
          )}
        </div>

        <AnimatePresence>
          {suggestions && (
            <motion.div
              initial={{ opacity: 0, y: 4 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -4 }}
              className="mt-2 px-3 py-2 rounded-[var(--radius-sm)] bg-[var(--color-accent-50)] border border-[var(--color-accent-100)]"
            >
              <div className="flex items-center gap-1.5 text-[var(--color-accent-700)] text-[10px] font-semibold uppercase tracking-wider mb-1.5">
                <Sparkles size={10} strokeWidth={2} />
                Advisor suggestions
                {suggestModel && (
                  <span className="font-mono normal-case tracking-normal text-[10px] text-[var(--color-paper-500)] ml-1">
                    · {suggestModel}
                  </span>
                )}
                <button
                  onClick={() => setSuggestions(null)}
                  className="ml-auto p-0.5 hover:bg-[var(--color-accent-100)] rounded-sm"
                >
                  <X size={11} />
                </button>
              </div>
              <div className="space-y-1.5">
                {suggestions.map((s, i) => (
                  <button
                    key={i}
                    onClick={() =>
                      addCode(
                        s.code,
                        "model-suggested-accepted",
                        s.rationale ?? undefined,
                        suggestModel ?? undefined,
                      )
                    }
                    className="w-full text-left px-2.5 py-1.5 rounded-[var(--radius-xs)] bg-white/60 hover:bg-white border border-transparent hover:border-[var(--color-accent-300)] transition-paper group"
                  >
                    <div className="flex items-center gap-2">
                      <Tag
                        size={10}
                        strokeWidth={2}
                        className="text-[var(--color-accent-500)]"
                      />
                      <span className="text-[12px] font-medium text-[var(--color-ink-900)]">
                        {s.code}
                      </span>
                      {s.is_new && (
                        <span className="text-[9px] font-semibold uppercase tracking-wider px-1 py-px rounded-sm bg-[var(--color-accent-300)] text-white">
                          new
                        </span>
                      )}
                      <Check
                        size={12}
                        className="ml-auto opacity-0 group-hover:opacity-100 text-[var(--color-ok-500)]"
                      />
                    </div>
                    {s.rationale && (
                      <p className="ml-5 mt-0.5 text-[11px] text-[var(--color-paper-600)] italic leading-snug">
                        {s.rationale}
                      </p>
                    )}
                  </button>
                ))}
              </div>
              <p className="mt-2 text-[10px] text-[var(--color-paper-500)] italic">
                Click any to accept. The local model suggests; you decide.
              </p>
            </motion.div>
          )}
        </AnimatePresence>

        {suggestError && (
          <p className="mt-2 text-[11px] text-[var(--color-error-500)]">{suggestError}</p>
        )}
      </div>

      <div className="flex items-center gap-1 shrink-0">
        <button
          onClick={() => setAdding(true)}
          className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] text-[var(--color-paper-600)] hover:text-[var(--color-ink-800)] transition-paper"
          title="Add code"
        >
          <Plus size={13} strokeWidth={2} />
        </button>
        <button
          onClick={requestSuggestions}
          disabled={suggesting}
          className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-accent-50)] text-[var(--color-accent-500)] transition-paper disabled:opacity-50"
          title="Suggest with local model"
        >
          {suggesting ? (
            <Loader2 size={13} className="animate-spin" />
          ) : (
            <Sparkles size={13} strokeWidth={1.75} />
          )}
        </button>
      </div>
    </div>
  );
}

function AppliedCode({
  coding,
  onRemove,
}: {
  coding: CodingRow;
  onRemove: () => void;
}) {
  const isModel = coding.source.startsWith("model-");
  return (
    <motion.span
      layout
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.9 }}
      className={cn(
        "group inline-flex items-center gap-1.5 pl-2 pr-1 py-0.5 rounded-full border text-[12px]",
        isModel
          ? "bg-[var(--color-accent-50)] border-[var(--color-accent-100)] text-[var(--color-accent-700)]"
          : "bg-[var(--color-ink-100)] border-[var(--color-ink-100)] text-[var(--color-ink-800)]",
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
