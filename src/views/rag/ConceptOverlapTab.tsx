import { useCallback, useEffect, useMemo, useState } from "react";
import { CircleDot, Loader2, RefreshCw } from "lucide-react";
import { api, type ArticleRow, type GlossaryTerm } from "../../lib/api";
import { cn } from "../../lib/utils";
import { parseArticleIds } from "../../lib/articleMeta";

const COLORS = ["var(--color-accent-500)", "var(--color-ok-500)", "var(--color-warn-500)"];

/** Concept-overlap Venn: pick 2–3 glossary concepts and see how many articles
 *  discuss each, and which they share. Overlap = articles referencing both. */
export function ConceptOverlapTab() {
  const [terms, setTerms] = useState<GlossaryTerm[]>([]);
  const [articlesById, setArticlesById] = useState<Map<number, ArticleRow>>(new Map());
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selected, setSelected] = useState<number[]>([]); // term ids, max 3
  const [query, setQuery] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [t, arts] = await Promise.all([
        api.listGlossaryTerms(),
        api.listArticles().catch(() => [] as ArticleRow[]),
      ]);
      setTerms(t);
      setArticlesById(new Map(arts.map((a) => [a.id, a])));
      setLoadError(null);
    } catch (e: any) {
      setLoadError(e?.message ?? String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  const atMax = selected.length >= 3;
  const toggle = (id: number) =>
    setSelected((prev) => {
      if (prev.includes(id)) return prev.filter((x) => x !== id);
      if (prev.length >= 3) return prev; // cap at 3 — unselected rows are disabled
      return [...prev, id];
    });

  const sets = useMemo(
    () =>
      selected
        .map((id) => terms.find((t) => t.id === id))
        .filter((t): t is GlossaryTerm => !!t)
        .map((t) => ({ term: t, ids: new Set(parseArticleIds(t.article_ids)) })),
    [selected, terms],
  );

  // Articles shared by ALL selected concepts (the central intersection).
  const sharedAll = useMemo(() => {
    if (sets.length < 2) return [];
    const [first, ...rest] = sets;
    return [...first.ids].filter((id) => rest.every((s) => s.ids.has(id)));
  }, [sets]);

  const q = query.trim().toLowerCase();
  const visibleTerms = q ? terms.filter((t) => t.term.toLowerCase().includes(q)) : terms;

  return (
    <div className="max-w-4xl mx-auto px-6 py-6">
      <div className="flex items-center gap-3 mb-4">
        <div className="flex-1">
          <h2 className="font-serif text-[22px] text-[var(--color-ink-900)] leading-tight">Concept overlap</h2>
          <p className="text-[12px] text-[var(--color-paper-500)] mt-0.5">
            Pick 2–3 concepts to see how the articles discussing them overlap.
          </p>
        </div>
        <button onClick={() => void load()} aria-label="Reload concepts" className="p-2 rounded-[var(--radius-xs)] text-[var(--color-paper-500)] hover:bg-[var(--color-paper-100)] transition-paper" title="Reload">
          {loading ? <Loader2 size={15} className="animate-spin" /> : <RefreshCw size={15} strokeWidth={1.75} />}
        </button>
      </div>

      {loadError ? (
        <div className="text-[13px] text-[var(--color-error-600)] py-10 text-center">
          Couldn’t load concepts: {loadError}
        </div>
      ) : terms.length === 0 && !loading ? (
        <div className="text-[13px] text-[var(--color-paper-500)] italic py-10 text-center">
          No concepts yet — build the Glossary first (in the Glossary tab) to extract concepts.
        </div>
      ) : (
        <div className="grid grid-cols-[260px_1fr] gap-6">
          {/* Concept picker */}
          <div>
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Filter concepts…"
              className="w-full mb-2 px-2.5 py-1.5 text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-400)]"
            />
            <div className="space-y-1 max-h-[60vh] overflow-y-auto pr-1">
              {visibleTerms.map((t) => {
                const idx = selected.indexOf(t.id);
                const on = idx >= 0;
                const count = parseArticleIds(t.article_ids).length;
                const blocked = !on && atMax;
                return (
                  <button
                    key={t.id}
                    onClick={() => toggle(t.id)}
                    disabled={blocked}
                    title={blocked ? "Deselect a concept first (max 3)" : undefined}
                    className={cn(
                      "w-full flex items-center gap-2 px-2.5 py-1.5 rounded-[var(--radius-xs)] text-left text-[12px] transition-paper border",
                      on
                        ? "bg-[var(--color-paper-50)] border-[var(--color-paper-300)] text-[var(--color-ink-800)]"
                        : blocked
                        ? "border-transparent text-[var(--color-paper-400)] opacity-50 cursor-not-allowed"
                        : "border-transparent hover:bg-[var(--color-paper-100)] text-[var(--color-paper-600)]",
                    )}
                  >
                    <span
                      className="w-2.5 h-2.5 rounded-full shrink-0 border"
                      style={{ background: on ? COLORS[idx] : "transparent", borderColor: on ? COLORS[idx] : "var(--color-paper-300)" }}
                    />
                    <span className="flex-1 min-w-0 truncate">{t.term}</span>
                    <span className="text-[10px] tabular-nums text-[var(--color-paper-400)]">{count}</span>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Venn + shared list */}
          <div>
            {sets.length < 2 ? (
              <div className="flex flex-col items-center justify-center text-center text-[13px] text-[var(--color-paper-500)] border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] py-16">
                <CircleDot size={28} strokeWidth={1.25} className="text-[var(--color-paper-300)] mb-2" />
                Select at least two concepts to compare.
              </div>
            ) : (
              <>
                <VennDiagram sets={sets} />
                <div className="mt-4">
                  <p className="text-[12px] font-semibold text-[var(--color-ink-800)] mb-1.5">
                    {sharedAll.length} article{sharedAll.length === 1 ? "" : "s"} discuss {sets.length === 2 ? "both" : "all"} concept{sets.length === 2 ? "" : "s"}
                  </p>
                  <div className="space-y-1">
                    {sharedAll.map((id) => {
                      const a = articlesById.get(id);
                      if (!a) return null;
                      return (
                        <div key={id} className="text-[12px] text-[var(--color-ink-700)] leading-snug line-clamp-2">
                          {a.title}
                          <span className="text-[var(--color-paper-400)]"> · {[a.authors, a.year].filter(Boolean).join(", ") || "—"}</span>
                        </div>
                      );
                    })}
                    {sharedAll.length === 0 && (
                      <p className="text-[12px] text-[var(--color-paper-400)] italic">These concepts don't co-occur in any single article.</p>
                    )}
                  </div>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function VennDiagram({ sets }: { sets: { term: GlossaryTerm; ids: Set<number> }[] }) {
  const count = (pred: (membership: boolean[]) => boolean) => {
    const all = new Set<number>();
    sets.forEach((s) => s.ids.forEach((id) => all.add(id)));
    let n = 0;
    all.forEach((id) => {
      const membership = sets.map((s) => s.ids.has(id));
      if (pred(membership)) n++;
    });
    return n;
  };

  if (sets.length === 2) {
    const aOnly = count((m) => m[0] && !m[1]);
    const bOnly = count((m) => !m[0] && m[1]);
    const ab = count((m) => m[0] && m[1]);
    const r = 92;
    return (
      <svg viewBox="0 0 380 230" className="w-full max-w-md mx-auto" role="img" aria-label={`Overlap of ${sets[0].term.term} (${aOnly} only) and ${sets[1].term.term} (${bOnly} only), ${ab} shared`}>
        <circle cx={150} cy={115} r={r} fill={COLORS[0]} fillOpacity={0.28} stroke={COLORS[0]} strokeWidth={1.5} />
        <circle cx={230} cy={115} r={r} fill={COLORS[1]} fillOpacity={0.28} stroke={COLORS[1]} strokeWidth={1.5} />
        <text x={108} y={120} textAnchor="middle" className="fill-[var(--color-ink-900)]" fontSize={20} fontWeight={600}>{aOnly}</text>
        <text x={272} y={120} textAnchor="middle" className="fill-[var(--color-ink-900)]" fontSize={20} fontWeight={600}>{bOnly}</text>
        <text x={190} y={120} textAnchor="middle" className="fill-[var(--color-ink-900)]" fontSize={20} fontWeight={700}>{ab}</text>
        <Legend sets={sets} />
      </svg>
    );
  }

  // 3 sets
  const m = (i: number, want: boolean) => (mb: boolean[]) => mb[i] === want;
  const and = (...ps: ((mb: boolean[]) => boolean)[]) => (mb: boolean[]) => ps.every((p) => p(mb));
  const aOnly = count(and(m(0, true), m(1, false), m(2, false)));
  const bOnly = count(and(m(0, false), m(1, true), m(2, false)));
  const cOnly = count(and(m(0, false), m(1, false), m(2, true)));
  const ab = count(and(m(0, true), m(1, true), m(2, false)));
  const ac = count(and(m(0, true), m(1, false), m(2, true)));
  const bc = count(and(m(0, false), m(1, true), m(2, true)));
  const abc = count(and(m(0, true), m(1, true), m(2, true)));
  const r = 84;
  return (
    <svg viewBox="0 0 380 300" className="w-full max-w-md mx-auto" role="img" aria-label={`Three-way overlap of ${sets.map((s) => s.term.term).join(", ")}; ${abc} shared by all`}>
      <circle cx={190} cy={108} r={r} fill={COLORS[0]} fillOpacity={0.26} stroke={COLORS[0]} strokeWidth={1.5} />
      <circle cx={148} cy={184} r={r} fill={COLORS[1]} fillOpacity={0.26} stroke={COLORS[1]} strokeWidth={1.5} />
      <circle cx={232} cy={184} r={r} fill={COLORS[2]} fillOpacity={0.26} stroke={COLORS[2]} strokeWidth={1.5} />
      <text x={190} y={78} textAnchor="middle" fontSize={17} fontWeight={600} className="fill-[var(--color-ink-900)]">{aOnly}</text>
      <text x={112} y={205} textAnchor="middle" fontSize={17} fontWeight={600} className="fill-[var(--color-ink-900)]">{bOnly}</text>
      <text x={268} y={205} textAnchor="middle" fontSize={17} fontWeight={600} className="fill-[var(--color-ink-900)]">{cOnly}</text>
      <text x={150} y={140} textAnchor="middle" fontSize={14} fontWeight={600} className="fill-[var(--color-ink-800)]">{ab}</text>
      <text x={230} y={140} textAnchor="middle" fontSize={14} fontWeight={600} className="fill-[var(--color-ink-800)]">{ac}</text>
      <text x={190} y={218} textAnchor="middle" fontSize={14} fontWeight={600} className="fill-[var(--color-ink-800)]">{bc}</text>
      <text x={190} y={172} textAnchor="middle" fontSize={15} fontWeight={700} className="fill-[var(--color-ink-900)]">{abc}</text>
      <Legend sets={sets} />
    </svg>
  );
}

function Legend({ sets }: { sets: { term: GlossaryTerm; ids: Set<number> }[] }) {
  return (
    <g>
      {sets.map((s, i) => (
        <g key={s.term.id} transform={`translate(8, ${i * 18 + 8})`}>
          <rect width={10} height={10} rx={2} fill={COLORS[i]} />
          <text x={16} y={9} fontSize={11} className="fill-[var(--color-paper-700)]">{s.term.term}</text>
        </g>
      ))}
    </g>
  );
}
