import { useCallback, useEffect, useRef, useState } from "react";
import { BookOpen, Loader2, Plus, RefreshCw, Search, Trash2, X } from "lucide-react";
import { confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
import { api, type ArticleRow, type GlossaryTerm } from "../../lib/api";
import { startGlossaryExtract, useGlossaryTask } from "../../lib/bgTasks";
import { parseArticleIds, parseAliases } from "../../lib/articleMeta";
import { GlossaryMerges } from "./GlossaryMerges";

export function GlossaryTab() {
  const [terms, setTerms] = useState<GlossaryTerm[]>([]);
  const [loading, setLoading] = useState(false);
  const task = useGlossaryTask();
  const extracting = task.running;
  const progress = task.progress;
  // Local error state — for non-extract failures like deletes. Extraction
  // errors come from the bg task store and we surface whichever is freshest.
  const [localError, setLocalError] = useState<string | null>(null);
  const error = task.error || localError;
  const [query, setQuery] = useState("");
  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [articlesById, setArticlesById] = useState<Map<number, ArticleRow>>(new Map());
  const [pdfPaths, setPdfPaths] = useState<Map<number, string>>(new Map());
  // termId of the row whose article-list popover is open. Independent from
  // `expandedId` so the user can peek at sources without expanding the
  // definition (and vice versa).
  const [openArticlesFor, setOpenArticlesFor] = useState<number | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [t, arts] = await Promise.all([
        api.listGlossaryTerms(),
        api.listArticles().catch(() => [] as ArticleRow[]),
      ]);
      setTerms(t);
      setArticlesById(new Map(arts.map((a) => [a.id, a])));
      // Look up which articles have PDFs so the title can be a link.
      if (arts.length > 0) {
        api
          .pdfPathsForArticles(arts.map((a) => a.id))
          .then((map) => {
            const next = new Map<number, string>();
            for (const [k, v] of Object.entries(map)) {
              next.set(Number(k), v);
            }
            setPdfPaths(next);
          })
          .catch(() => setPdfPaths(new Map()));
      }
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const extract = useCallback((mode: "rebuild" | "update") => {
    startGlossaryExtract(mode);
  }, []);

  // Reload the term list whenever an extraction run finishes, regardless of
  // whether the user was on this tab when it completed.
  useEffect(() => {
    if (task.finishedAt) {
      void load();
    }
  }, [task.finishedAt, load]);

  const deleteTerm = async (id: number) => {
    const term = terms.find((t) => t.id === id);
    const ok = await confirmDialog(
      `Delete the concept “${term?.term ?? "this term"}”? It was extracted from the corpus and can only be recovered by re-running the glossary.`,
      { title: "Delete concept?", kind: "warning", okLabel: "Delete", cancelLabel: "Keep" },
    );
    if (!ok) return;
    try {
      await api.deleteGlossaryTerm(id);
      setTerms((prev) => prev.filter((t) => t.id !== id));
      if (expandedId === id) setExpandedId(null);
    } catch (e: unknown) {
      setLocalError(e instanceof Error ? e.message : String(e));
    }
  };

  const filtered = terms.filter((t) => {
    const q = query.toLowerCase();
    if (t.term.toLowerCase().includes(q)) return true;
    if (t.definition.toLowerCase().includes(q)) return true;
    return parseAliases(t.aliases).some((a) => a.toLowerCase().includes(q));
  });

  // A–Z index: the first term under each letter is the jump target.
  const listRef = useRef<HTMLDivElement>(null);
  const firstIdByLetter = new Map<string, number>();
  for (const t of filtered) {
    const l = indexLetter(t.term);
    if (!firstIdByLetter.has(l)) firstIdByLetter.set(l, t.id);
  }
  const jumpTo = (letter: string) => {
    const id = firstIdByLetter.get(letter);
    if (id === undefined) return;
    listRef.current
      ?.querySelector(`[data-term-id="${id}"]`)
      ?.scrollIntoView({ behavior: "smooth", block: "start" });
  };

  return (
    <div className="h-full flex flex-col">
      {/* Toolbar */}
      <div className="flex items-center gap-3 px-5 py-3 border-b border-[var(--color-paper-200)] bg-white shrink-0">
        <div className="flex-1 relative">
          <Search size={13} strokeWidth={1.75} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--color-paper-400)]" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search terms…"
            className="w-full pl-8 pr-3 py-1.5 text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-400)] transition-paper"
          />
          {query && (
            <button onClick={() => setQuery("")} className="absolute right-2.5 top-1/2 -translate-y-1/2 text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)]">
              <X size={11} strokeWidth={2} />
            </button>
          )}
        </div>
        <span className="text-[11px] text-[var(--color-paper-400)] shrink-0">
          {terms.length} term{terms.length !== 1 ? "s" : ""}
        </span>
        {terms.length > 0 && (
          <button
            onClick={() => extract("update")}
            disabled={extracting}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] bg-white border border-[var(--color-paper-300)] text-[var(--color-ink-800)] text-[11px] font-semibold hover:bg-[var(--color-paper-100)] disabled:opacity-50 transition-paper shrink-0"
            title="Extract terms only for articles not yet processed"
          >
            {extracting
              ? <Loader2 size={11} className="animate-spin" />
              : <Plus size={11} strokeWidth={2.25} />}
            Update
          </button>
        )}
        <button
          onClick={() => extract("rebuild")}
          disabled={extracting}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] bg-[var(--color-ink-800)] text-white text-[11px] font-semibold hover:bg-[var(--color-ink-700)] disabled:opacity-50 transition-paper shrink-0"
          title={terms.length > 0 ? "Discard everything and rebuild from scratch" : "Run the first extraction"}
        >
          {extracting
            ? <Loader2 size={11} className="animate-spin" />
            : <RefreshCw size={11} strokeWidth={2} />
          }
          {extracting ? "Extracting…" : terms.length > 0 ? "Re-extract" : "Extract glossary"}
        </button>
      </div>

      {/* Progress bar during extraction */}
      {extracting && progress && (
        <div className="px-5 py-2 bg-[var(--color-accent-50)] border-b border-[var(--color-accent-100)] shrink-0">
          <div className="flex justify-between text-[11px] text-[var(--color-paper-500)] mb-1">
            <span className="truncate max-w-xs">{progress.article_title || "Starting…"}</span>
            <span className="font-mono">{progress.done}/{progress.total}</span>
          </div>
          <div className="h-1 rounded-full bg-[var(--color-paper-200)] overflow-hidden">
            <div
              className="h-full rounded-full bg-[var(--color-accent-500)] transition-all duration-300"
              style={{ width: progress.total > 0 ? `${Math.round((progress.done / progress.total) * 100)}%` : "0%" }}
            />
          </div>
        </div>
      )}

      {!extracting && <GlossaryMerges termCount={terms.length} onChanged={load} />}

      {/* Error */}
      {error && (
        <p className="px-5 py-2 text-[12px] text-[var(--color-error-600)] bg-[var(--color-error-50)] border-b border-[var(--color-error-100)] shrink-0">
          {error}
        </p>
      )}

      {/* Term list + A–Z index */}
      <div className="flex-1 flex min-h-0">
      <div ref={listRef} className="flex-1 overflow-y-auto pb-12">
        {loading && (
          <div className="flex items-center justify-center py-16">
            <Loader2 size={20} className="animate-spin text-[var(--color-paper-400)]" />
          </div>
        )}

        {!loading && terms.length === 0 && (
          <div className="flex flex-col items-center justify-center gap-3 py-16 text-center px-8">
            <BookOpen size={32} strokeWidth={1.25} className="text-[var(--color-paper-300)]" />
            <p className="text-[13px] font-semibold text-[var(--color-ink-800)]">No glossary yet</p>
            <p className="text-[12px] text-[var(--color-paper-500)] max-w-xs">
              Click "Extract glossary" to have the LLM identify domain-specific terms and definitions from your indexed articles.
            </p>
          </div>
        )}

        {!loading && filtered.length === 0 && terms.length > 0 && (
          <p className="px-5 py-8 text-center text-[12px] text-[var(--color-paper-400)]">
            No terms match "{query}"
          </p>
        )}

        {!loading && filtered.length > 0 && (
          <div className="divide-y divide-[var(--color-paper-100)]">
            {filtered.map((term) => {
              const articleIds = parseArticleIds(term.article_ids);
              const articleCount = articleIds.length;
              const isExpanded = expandedId === term.id;

              const sources: ArticleRow[] = articleIds
                .map((id) => articlesById.get(id))
                .filter((a): a is ArticleRow => !!a);
              const isShowingSources = openArticlesFor === term.id;
              return (
                <div key={term.id} data-term-id={term.id} className="group px-5 py-3 hover:bg-[var(--color-paper-50)] transition-paper scroll-mt-1">
                  <div className="flex items-start gap-2">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-0.5 flex-wrap">
                        <span
                          className="text-[13px] font-semibold text-[var(--color-ink-900)] cursor-pointer"
                          onClick={() => setExpandedId(isExpanded ? null : term.id)}
                        >
                          {term.term}
                        </span>
                        {(() => {
                          const aliases = parseAliases(term.aliases);
                          if (aliases.length === 0) return null;
                          return (
                            <span className="text-[11px] text-[var(--color-paper-500)] italic">
                              also known as {aliases.join(", ")}
                            </span>
                          );
                        })()}
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            setOpenArticlesFor(isShowingSources ? null : term.id);
                          }}
                          className={
                            "text-[10px] font-medium px-1.5 py-0.5 rounded-full transition-paper " +
                            (isShowingSources
                              ? "bg-[var(--color-accent-100)] text-[var(--color-accent-700)]"
                              : "bg-[var(--color-paper-100)] text-[var(--color-paper-500)] hover:bg-[var(--color-paper-200)] hover:text-[var(--color-ink-700)]")
                          }
                          title="Show source articles"
                        >
                          {articleCount} article{articleCount !== 1 ? "s" : ""}
                        </button>
                      </div>
                      <p
                        className={`text-[12px] text-[var(--color-paper-600)] leading-relaxed cursor-pointer ${isExpanded ? "" : "line-clamp-2"}`}
                        onClick={() => setExpandedId(isExpanded ? null : term.id)}
                      >
                        {term.definition}
                      </p>
                      {isShowingSources && (
                        <div className="mt-2 rounded-[var(--radius-xs)] border border-[var(--color-paper-200)] bg-white divide-y divide-[var(--color-paper-100)]">
                          {sources.length === 0 ? (
                            <p className="px-3 py-2 text-[11px] text-[var(--color-paper-400)] italic">
                              {articleIds.length} source{articleIds.length === 1 ? "" : "s"} recorded but no matching article found in the project.
                            </p>
                          ) : (
                            sources.map((a) => {
                              const pdfPath = pdfPaths.get(a.id);
                              return (
                                <div key={a.id} className="px-3 py-2 text-[12px]">
                                  {pdfPath ? (
                                    <button
                                      type="button"
                                      onClick={() => api.openPdfPath(pdfPath).catch(() => {})}
                                      className="font-medium text-[var(--color-accent-700)] hover:text-[var(--color-accent-800)] hover:underline leading-snug text-left"
                                      title={`Open ${pdfPath.split("/").pop() ?? "PDF"}`}
                                    >
                                      {a.title}
                                    </button>
                                  ) : (
                                    <div className="font-medium text-[var(--color-ink-800)] leading-snug">{a.title}</div>
                                  )}
                                  <div className="text-[11px] text-[var(--color-paper-500)] mt-0.5">
                                    {[a.authors, a.year, a.journal].filter(Boolean).join(" · ") || "—"}
                                    {!pdfPath && (
                                      <span className="ml-2 italic text-[var(--color-paper-400)]">no PDF attached</span>
                                    )}
                                  </div>
                                </div>
                              );
                            })
                          )}
                        </div>
                      )}
                    </div>
                    <button
                      onClick={() => deleteTerm(term.id)}
                      className="p-1 rounded-[var(--radius-xs)] text-[var(--color-paper-300)] hover:text-[var(--color-error-500)] opacity-0 group-hover:opacity-100 transition-paper shrink-0 mt-0.5"
                    >
                      <Trash2 size={12} strokeWidth={1.75} />
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
      {!loading && filtered.length > 0 && (
        <nav
          aria-label="Jump to letter"
          className="shrink-0 w-7 flex flex-col items-center justify-center gap-px py-2 border-l border-[var(--color-paper-200)] bg-white overflow-y-auto"
        >
          {INDEX_LETTERS.map((l) => {
            const has = firstIdByLetter.has(l);
            return (
              <button
                key={l}
                type="button"
                onClick={() => jumpTo(l)}
                disabled={!has}
                className={
                  has
                    ? "w-5 text-[10px] leading-[15px] font-semibold text-[var(--color-accent-600)] rounded-[var(--radius-xs)] hover:bg-[var(--color-accent-50)]"
                    : "w-5 text-[10px] leading-[15px] text-[var(--color-paper-300)] cursor-default"
                }
                title={has ? `Jump to ${l}` : `No terms under ${l}`}
              >
                {l}
              </button>
            );
          })}
        </nav>
      )}
      </div>
    </div>
  );
}

const INDEX_LETTERS = [..."ABCDEFGHIJKLMNOPQRSTUVWXYZ", "#"];

/** Index letter for a term: its first letter (accents ignored), or "#". */
function indexLetter(term: string): string {
  const c = term.trim().normalize("NFD").replace(/[\u0300-\u036f]/g, "").charAt(0).toUpperCase();
  return c >= "A" && c <= "Z" ? c : "#";
}
