import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { BookOpen, Loader2, RefreshCw, Search, Trash2, X } from "lucide-react";
import { api, type GlossaryProgress, type GlossaryTerm } from "../../lib/api";

export function GlossaryTab() {
  const [terms, setTerms] = useState<GlossaryTerm[]>([]);
  const [loading, setLoading] = useState(false);
  const [extracting, setExtracting] = useState(false);
  const [progress, setProgress] = useState<GlossaryProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [expandedId, setExpandedId] = useState<number | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const t = await api.listGlossaryTerms();
      setTerms(t);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const extract = useCallback(async () => {
    setExtracting(true);
    setError(null);
    setProgress(null);

    const unlisten = await listen<GlossaryProgress>("glossary:progress", (e) => {
      setProgress(e.payload);
    });

    try {
      await api.extractGlossary();
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      unlisten();
      setExtracting(false);
      setProgress(null);
    }
  }, [load]);

  const deleteTerm = async (id: number) => {
    await api.deleteGlossaryTerm(id);
    setTerms((prev) => prev.filter((t) => t.id !== id));
    if (expandedId === id) setExpandedId(null);
  };

  const filtered = terms.filter((t) =>
    t.term.toLowerCase().includes(query.toLowerCase()) ||
    t.definition.toLowerCase().includes(query.toLowerCase()),
  );

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
        <button
          onClick={extract}
          disabled={extracting}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] bg-[var(--color-ink-800)] text-white text-[11px] font-semibold hover:bg-[var(--color-ink-700)] disabled:opacity-50 transition-paper shrink-0"
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

      {/* Error */}
      {error && (
        <p className="px-5 py-2 text-[12px] text-[var(--color-error-600)] bg-[var(--color-error-50)] border-b border-[var(--color-error-100)] shrink-0">
          {error}
        </p>
      )}

      {/* Term list */}
      <div className="flex-1 overflow-y-auto">
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
              const articleCount = (() => {
                try { return JSON.parse(term.article_ids).length; } catch { return 0; }
              })();
              const isExpanded = expandedId === term.id;

              return (
                <div key={term.id} className="group px-5 py-3 hover:bg-[var(--color-paper-50)] transition-paper">
                  <div className="flex items-start gap-2">
                    <div
                      className="flex-1 min-w-0 cursor-pointer"
                      onClick={() => setExpandedId(isExpanded ? null : term.id)}
                    >
                      <div className="flex items-center gap-2 mb-0.5">
                        <span className="text-[13px] font-semibold text-[var(--color-ink-900)]">{term.term}</span>
                        <span className="text-[10px] font-medium px-1.5 py-0.5 rounded-full bg-[var(--color-paper-100)] text-[var(--color-paper-500)]">
                          {articleCount} article{articleCount !== 1 ? "s" : ""}
                        </span>
                      </div>
                      <p className={`text-[12px] text-[var(--color-paper-600)] leading-relaxed ${isExpanded ? "" : "line-clamp-2"}`}>
                        {term.definition}
                      </p>
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
    </div>
  );
}
