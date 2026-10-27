import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Database,
  FileText,
  Loader2,
  Pencil,
  Plus,
  RefreshCw,
  Search,
  Sparkles,
  Upload,
  X,
} from "lucide-react";
import { api, type ArticleFields, type ArticleRow, type BuildRagProgress, type SemanticHit } from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { cn } from "../../lib/utils";
import { parseSources } from "../../lib/articleMeta";

function blankFields(): ArticleFields {
  return { title: "", authors: null, year: null, journal: null, doi: null, url: null, abstract_text: null, source_databases: [] };
}

function fieldsFromArticle(a: ArticleRow): ArticleFields {
  return {
    title: a.title,
    authors: a.authors,
    year: a.year,
    journal: a.journal,
    doi: a.doi,
    url: a.url,
    abstract_text: a.abstract_text,
    source_databases: parseSources(a.source_databases),
  };
}

/** A list of all imported literature, with the source database, inline editing,
 *  and an "Add literature" flow (manual / DOI lookup / attach PDF). */
export function LibraryTab() {
  const [articles, setArticles] = useState<ArticleRow[]>([]);
  const [pdfPaths, setPdfPaths] = useState<Map<number, string>>(new Map());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  // Search mode: "traditional" (word match, the default) or "ai" (by meaning).
  const [mode, setMode] = useState<"traditional" | "ai">("traditional");
  const [resultCount, setResultCount] = useState(25);
  const [aiHits, setAiHits] = useState<SemanticHit[] | null>(null);
  const [aiState, setAiState] = useState<
    | { kind: "idle" }
    | { kind: "indexing"; done: number; total: number }
    | { kind: "searching" }
    | { kind: "done"; model: string; newlyIndexed: number; pdfsNotIndexed: number }
    | { kind: "fallback"; reason: string }
  >({ kind: "idle" });
  const searchSeq = useRef(0);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [adding, setAdding] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const arts = await api.listArticles();
      setArticles(arts);
      if (arts.length > 0) {
        api.pdfPathsForArticles(arts.map((a) => a.id))
          .then((rec) => setPdfPaths(new Map(Object.entries(rec).map(([k, v]) => [Number(k), v]))))
          .catch(() => setPdfPaths(new Map()));
      } else {
        setPdfPaths(new Map());
      }
      setError(null);
    } catch (e: any) {
      setError(e?.message ?? String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  // Remember the researcher's search preferences per project.
  useEffect(() => {
    api.getSettings()
      .then((s) => {
        if (s["library.search_mode"] === "ai") setMode("ai");
        const n = s["library.ai_result_count"];
        if (typeof n === "number") setResultCount(n);
      })
      .catch(() => {});
  }, []);
  const chooseMode = (m: "traditional" | "ai") => {
    setMode(m);
    api.setSetting("library.search_mode", m).catch(() => {});
  };
  const chooseResultCount = (n: number) => {
    setResultCount(n);
    api.setSetting("library.ai_result_count", n).catch(() => {});
  };

  // AI-enhanced search runs when typing pauses. If it can't run (no embedding
  // model, LM Studio unreachable) it falls back to traditional search.
  useEffect(() => {
    if (mode !== "ai") return;
    const q = query.trim();
    if (!q) { setAiHits(null); setAiState({ kind: "idle" }); return; }
    const seq = ++searchSeq.current;
    const timer = setTimeout(async () => {
      setAiState({ kind: "searching" });
      const unlisten = await listen<BuildRagProgress>("rag:progress", (e) => {
        if (seq === searchSeq.current && e.payload.phase === "abstracts" && e.payload.total > 0) {
          setAiState({ kind: "indexing", done: e.payload.done, total: e.payload.total });
        }
      });
      try {
        const r = await api.semanticSearchArticles(q);
        if (seq !== searchSeq.current) return;
        setAiHits(r.hits);
        setAiState({ kind: "done", model: r.embedding_model, newlyIndexed: r.newly_indexed, pdfsNotIndexed: r.pdfs_not_indexed });
      } catch (e) {
        if (seq !== searchSeq.current) return;
        const msg = e instanceof Error ? e.message : String(e);
        setAiHits(null);
        setAiState({
          kind: "fallback",
          reason: msg.includes("Connection refused") || msg.includes("error sending request")
            ? "LM Studio isn't reachable"
            : msg,
        });
      } finally {
        unlisten();
      }
    }, 500);
    return () => clearTimeout(timer);
  }, [mode, query]);

  const q = query.trim().toLowerCase();
  const traditional = q
    ? articles.filter((a) =>
        [a.title, a.authors, a.journal, a.doi, ...parseSources(a.source_databases)]
          .filter(Boolean)
          .some((f) => (f as string).toLowerCase().includes(q)),
      )
    : articles;
  const byId = new Map(articles.map((a) => [a.id, a]));
  const hitById = new Map((aiHits ?? []).map((h) => [h.article_id, h]));
  const useAi = mode === "ai" && q.length > 0 && aiHits !== null && aiState.kind === "done";
  const filtered = useAi
    ? aiHits!.slice(0, resultCount).map((h) => byId.get(h.article_id)).filter((a): a is ArticleRow => !!a)
    : traditional;
  const weakBest = useAi && aiHits!.length > 0 && aiHits![0].score < 0.45;

  return (
    <div className="max-w-4xl mx-auto px-6 py-6">
      {/* Header */}
      <div className="flex items-center gap-3 mb-4">
        <div className="flex-1">
          <h2 className="font-serif text-[22px] text-[var(--color-ink-900)] leading-tight">Library</h2>
          <p className="text-[12px] text-[var(--color-paper-500)] mt-0.5">
            {articles.length} item{articles.length === 1 ? "" : "s"} imported · edit fields or add literature.
          </p>
        </div>
        <button
          onClick={() => void load()}
          className="p-2 rounded-[var(--radius-xs)] text-[var(--color-paper-500)] hover:bg-[var(--color-paper-100)] transition-paper"
          title="Reload"
        >
          {loading ? <Loader2 size={15} className="animate-spin" /> : <RefreshCw size={15} strokeWidth={1.75} />}
        </button>
        <Button variant="primary" size="sm" onClick={() => { setAdding(true); setEditingId(null); }}>
          <Plus size={14} strokeWidth={2} /> Add literature
        </Button>
      </div>

      {/* Search */}
      <div className="flex items-center gap-2 mb-2">
        <div className="inline-flex rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] bg-white p-0.5 text-[12px]" role="radiogroup" aria-label="Search mode">
          {(["traditional", "ai"] as const).map((m) => (
            <button
              key={m}
              type="button"
              role="radio"
              aria-checked={mode === m}
              onClick={() => chooseMode(m)}
              className={cn(
                "inline-flex items-center gap-1 px-2.5 py-1 rounded-[var(--radius-xs)] transition-paper",
                mode === m ? "bg-[var(--color-ink-800)] text-white" : "text-[var(--color-paper-600)] hover:text-[var(--color-ink-800)]",
              )}
              title={m === "traditional"
                ? "Match the exact words in title, author, journal, DOI or database"
                : "Find papers by meaning across title, abstract and indexed PDF text, using your embedding model"}
            >
              {m === "ai" && <Sparkles size={11} strokeWidth={2} />}
              {m === "traditional" ? "Traditional search" : "AI-enhanced search"}
            </button>
          ))}
        </div>
        {mode === "ai" && (
          <label className="ml-auto flex items-center gap-2 text-[11px] text-[var(--color-paper-600)]">
            Show top
            <input
              type="range"
              min={5}
              max={Math.max(5, Math.ceil(articles.length / 5) * 5)}
              step={5}
              value={Math.min(resultCount, Math.max(5, Math.ceil(articles.length / 5) * 5))}
              onChange={(e) => chooseResultCount(Number(e.target.value))}
              aria-label="Number of results to show"
            />
            <span className="tabular-nums w-10">{resultCount >= articles.length ? "all" : resultCount}</span>
          </label>
        )}
      </div>
      <div className="relative mb-2">
        <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-paper-400)]" />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={mode === "ai"
            ? "Describe what you're looking for, e.g. students relying on chatbots instead of thinking"
            : "Search title, author, journal, DOI, database…"}
          className="w-full pl-9 pr-3 py-2 text-[13px] rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] bg-white focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-400)]"
        />
      </div>
      {mode === "ai" && q.length > 0 && (
        <div className="mb-4 text-[11px] leading-snug">
          {aiState.kind === "searching" && (
            <p className="text-[var(--color-paper-500)] inline-flex items-center gap-1.5"><Loader2 size={11} className="animate-spin" /> Searching by meaning…</p>
          )}
          {aiState.kind === "indexing" && (
            <p className="text-[var(--color-paper-500)] inline-flex items-center gap-1.5">
              <Loader2 size={11} className="animate-spin" />
              {aiState.total} paper{aiState.total === 1 ? " isn't" : "s aren't"} indexed yet. Indexing their titles and abstracts first… {aiState.done}/{aiState.total}
            </p>
          )}
          {aiState.kind === "fallback" && (
            <p className="text-[var(--color-warn-700)]">AI-enhanced search couldn't run ({aiState.reason}), so these are traditional search results.</p>
          )}
          {aiState.kind === "done" && (
            <p className="text-[var(--color-paper-500)]">
              Ranked by similarity to your description ({aiState.model})
              {aiState.newlyIndexed > 0 && ` · indexed ${aiState.newlyIndexed} new title${aiState.newlyIndexed === 1 ? "" : "s"} & abstract${aiState.newlyIndexed === 1 ? "" : "s"} first`}
              {aiState.pdfsNotIndexed > 0 && ` · ${aiState.pdfsNotIndexed} PDF${aiState.pdfsNotIndexed === 1 ? " isn't" : "s aren't"} indexed yet, so ${aiState.pdfsNotIndexed === 1 ? "its" : "their"} full text isn't searched. Run Build index in Chat to include ${aiState.pdfsNotIndexed === 1 ? "it" : "them"}.`}
              {weakBest && <span className="block text-[var(--color-warn-700)]">Even the best match is weak. Your library may not cover this topic.</span>}
            </p>
          )}
        </div>
      )}
      {!(mode === "ai" && q.length > 0) && <div className="mb-2" />}

      {error && <p className="mb-3 text-[12px] text-[var(--color-error-600)]">{error}</p>}

      {/* Add form */}
      {adding && (
        <div className="mb-4">
          <ArticleForm
            title="Add literature"
            allowPdf
            onCancel={() => setAdding(false)}
            onSaved={async () => { setAdding(false); await load(); }}
          />
        </div>
      )}

      {/* List */}
      <div className="space-y-3">
        {filtered.length === 0 && !loading && (
          <p className="text-[13px] text-[var(--color-paper-500)] italic py-8 text-center">
            {articles.length === 0 ? "No literature yet — add some with the button above." : "No matches."}
          </p>
        )}
        {filtered.map((a) =>
          editingId === a.id ? (
            <ArticleForm
              key={a.id}
              title="Edit article"
              initial={fieldsFromArticle(a)}
              articleId={a.id}
              onCancel={() => setEditingId(null)}
              onSaved={async () => { setEditingId(null); await load(); }}
            />
          ) : (
            <LibraryCard
              key={a.id}
              article={a}
              match={useAi ? hitById.get(a.id) ?? null : null}
              pdfPath={pdfPaths.get(a.id) ?? null}
              onEdit={() => { setEditingId(a.id); setAdding(false); }}
              onChanged={load}
              onError={setError}
            />
          ),
        )}
      </div>
    </div>
  );
}

function LibraryCard({
  article,
  match = null,
  pdfPath,
  onEdit,
  onChanged,
  onError,
}: {
  article: ArticleRow;
  /** The AI-enhanced search match for this paper, when ranked by meaning. */
  match?: SemanticHit | null;
  pdfPath: string | null;
  onEdit: () => void;
  onChanged: () => Promise<void>;
  onError: (msg: string) => void;
}) {
  const [attaching, setAttaching] = useState(false);
  const sources = parseSources(article.source_databases);

  const attachPdf = async () => {
    const selected = await openDialog({ multiple: false, filters: [{ name: "PDF", extensions: ["pdf"] }], title: "Select PDF for this article" });
    if (!selected || Array.isArray(selected)) return;
    setAttaching(true);
    try {
      await api.dropPdfForArticle(selected as string, article.id);
      await onChanged();
    } catch (e: any) {
      onError(`Couldn't attach PDF: ${e?.message ?? String(e)}`);
    } finally {
      setAttaching(false);
    }
  };

  return (
    <Card className="overflow-hidden">
      <div className="px-5 py-4 flex gap-4 items-start">
        <div className="flex-1 min-w-0">
          <h3 className="font-serif text-[18px] leading-[1.35] text-[var(--color-ink-900)] mb-1.5">{article.title}</h3>
          <div className="space-y-0.5">
            {(article.authors || article.year) && (
              <p className="text-[12px] text-[var(--color-paper-700)]">
                {article.authors && <span>{article.authors}</span>}
                {article.authors && article.year && <span className="mx-1.5 text-[var(--color-paper-300)]">·</span>}
                {article.year && <span className="tabular-nums font-mono">{article.year}</span>}
              </p>
            )}
            {article.journal && <p className="text-[12px] italic text-[var(--color-paper-500)]">{article.journal}</p>}
            {article.doi && (
              <button
                onClick={() => openUrl(`https://doi.org/${article.doi}`).catch(() => {})}
                className="text-[11px] font-mono text-[var(--color-accent-700)] hover:underline"
              >
                DOI: {article.doi}
              </button>
            )}
          </div>
          {match && (
            <div
              className="mt-2 text-[11px] leading-snug text-[var(--color-paper-600)]"
              title="Cosine similarity to your description. PDF passages are adjusted for ranking, because the best of many passages scores higher than an abstract on average."
            >
              <span className="inline-flex items-center gap-1 font-medium text-[var(--color-accent-700)]">
                <Sparkles size={10} strokeWidth={2} /> {match.score.toFixed(2)}
              </span>
              <span className="text-[var(--color-paper-400)]">
                {match.abstract_score !== null && ` · abstract ${match.abstract_score.toFixed(2)}`}
                {match.passage_score !== null && ` · best passage ${match.passage_score.toFixed(2)}`}
              </span>
              {match.passage && match.passage_score !== null && match.passage_score > (match.abstract_score ?? -1) && (
                <p className="mt-1 italic text-[var(--color-paper-500)] line-clamp-2">“{match.passage}…”</p>
              )}
            </div>
          )}
          {/* Source databases + status */}
          <div className="flex flex-wrap items-center gap-1.5 mt-2.5">
            {sources.length === 0 ? (
              <span className="inline-flex items-center gap-1 text-[10px] text-[var(--color-paper-400)]">
                <Database size={10} strokeWidth={1.75} /> source unknown
              </span>
            ) : (
              sources.map((s) => (
                <span key={s} className="inline-flex items-center gap-1 text-[10px] font-medium text-[var(--color-ink-700)] bg-[var(--color-paper-100)] border border-[var(--color-paper-200)] rounded-full px-2 py-0.5">
                  <Database size={10} strokeWidth={1.75} className="text-[var(--color-paper-500)]" /> {s}
                </span>
              ))
            )}
            <span className="text-[10px] uppercase tracking-wider text-[var(--color-paper-400)] ml-1">{article.status}</span>
          </div>
        </div>

        {/* PDF + edit */}
        <div className="shrink-0 w-[120px] flex flex-col gap-2">
          {pdfPath ? (
            <button
              onClick={() => api.openPdfPath(pdfPath).catch(() => {})}
              className="w-full flex flex-col items-center justify-center gap-1.5 px-3 py-3 rounded-[var(--radius-sm)] bg-[var(--color-ok-50)] border border-[var(--color-ok-200)] hover:bg-[var(--color-ok-100)] text-[var(--color-ok-700)] transition-paper"
            >
              <FileText size={18} strokeWidth={1.5} />
              <span className="text-[11px] font-medium">Open PDF</span>
            </button>
          ) : (
            <button
              onClick={attachPdf}
              disabled={attaching}
              className="w-full flex flex-col items-center justify-center gap-1.5 px-3 py-3 rounded-[var(--radius-sm)] border border-dashed border-[var(--color-paper-300)] text-[var(--color-paper-500)] hover:border-[var(--color-ink-400)] hover:text-[var(--color-ink-700)] transition-paper"
            >
              {attaching ? <Loader2 size={16} className="animate-spin" /> : <Upload size={16} strokeWidth={1.5} />}
              <span className="text-[11px] font-medium">Attach PDF</span>
            </button>
          )}
          <button
            onClick={onEdit}
            className="w-full inline-flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[11px] font-medium text-[var(--color-paper-600)] hover:bg-[var(--color-paper-100)] transition-paper"
          >
            <Pencil size={12} strokeWidth={1.75} /> Edit
          </button>
        </div>
      </div>
    </Card>
  );
}

function ArticleForm({
  title,
  initial,
  articleId,
  allowPdf,
  onCancel,
  onSaved,
}: {
  title: string;
  initial?: ArticleFields;
  articleId?: number;
  allowPdf?: boolean;
  onCancel: () => void;
  onSaved: () => Promise<void>;
}) {
  const [f, setF] = useState<ArticleFields>(initial ?? blankFields());
  const [doi, setDoi] = useState(initial?.doi ?? "");
  const [sourceText, setSourceText] = useState((initial?.source_databases ?? []).join(", "));
  const [pdfPath, setPdfPath] = useState<string | null>(null);
  const [looking, setLooking] = useState(false);
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const set = <K extends keyof ArticleFields>(k: K, v: ArticleFields[K]) => setF((p) => ({ ...p, [k]: v }));

  const lookUp = async () => {
    const d = doi.trim();
    if (!d) return;
    setLooking(true);
    setErr(null);
    try {
      const r = await api.lookupDoi(d);
      const found = r.title || r.authors || r.year || r.journal || r.abstract_text;
      setF((p) => ({
        ...p,
        title: r.title ?? p.title,
        authors: r.authors ?? p.authors,
        year: r.year ?? p.year,
        journal: r.journal ?? p.journal,
        abstract_text: r.abstract_text ?? p.abstract_text,
        doi: d,
      }));
      if (!found) setErr("No metadata found for that DOI — enter the fields manually.");
    } catch (e: any) {
      setErr(e?.message ?? String(e));
    } finally {
      setLooking(false);
    }
  };

  const pickPdf = async () => {
    const selected = await openDialog({ multiple: false, filters: [{ name: "PDF", extensions: ["pdf"] }], title: "Select PDF" });
    if (selected && !Array.isArray(selected)) setPdfPath(selected as string);
  };

  const save = async () => {
    if (!f.title.trim()) { setErr("A title is required."); return; }
    setSaving(true);
    setErr(null);
    const fields: ArticleFields = {
      ...f,
      doi: doi.trim() || null,
      source_databases: sourceText.split(",").map((s) => s.trim()).filter(Boolean),
    };
    try {
      if (articleId != null) {
        await api.updateArticleFields(articleId, fields);
      } else {
        const id = await api.createArticle(fields);
        if (pdfPath) await api.dropPdfForArticle(pdfPath, id).catch(() => {});
      }
      await onSaved();
    } catch (e: any) {
      setErr(e?.message ?? String(e));
      setSaving(false);
    }
  };

  const input = "w-full px-2.5 py-1.5 text-[13px] rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-400)]";
  const label = "text-[11px] font-semibold uppercase tracking-wider text-[var(--color-paper-500)] mb-1 block";

  return (
    <Card className="overflow-hidden border-[var(--color-accent-300)]">
      <div className="px-5 py-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-[14px] font-semibold text-[var(--color-ink-900)]">{title}</h3>
          <button onClick={onCancel} className="p-1 rounded-full text-[var(--color-paper-400)] hover:bg-[var(--color-paper-100)]"><X size={14} /></button>
        </div>

        {/* DOI lookup row */}
        <div className="mb-3">
          <span className={label}>Look up by DOI (optional)</span>
          <div className="flex gap-2">
            <input value={doi} onChange={(e) => setDoi(e.target.value)} placeholder="10.1016/j.example.2024.01.001" className={cn(input, "flex-1 font-mono")} />
            <Button variant="secondary" size="sm" onClick={lookUp} disabled={looking || !doi.trim()}>
              {looking ? <Loader2 size={13} className="animate-spin" /> : "Look up"}
            </Button>
          </div>
        </div>

        <div className="space-y-3">
          <div>
            <span className={label}>Title</span>
            <input value={f.title} onChange={(e) => set("title", e.target.value)} className={input} />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <span className={label}>Authors</span>
              <input value={f.authors ?? ""} onChange={(e) => set("authors", e.target.value || null)} className={input} />
            </div>
            <div>
              <span className={label}>Year</span>
              <input
                type="number"
                value={f.year ?? ""}
                onChange={(e) => set("year", e.target.value ? Number(e.target.value) : null)}
                className={input}
              />
            </div>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <span className={label}>Journal</span>
              <input value={f.journal ?? ""} onChange={(e) => set("journal", e.target.value || null)} className={input} />
            </div>
            <div>
              <span className={label}>URL</span>
              <input value={f.url ?? ""} onChange={(e) => set("url", e.target.value || null)} className={input} />
            </div>
          </div>
          <div>
            <span className={label}>Source database(s) — comma separated</span>
            <input value={sourceText} onChange={(e) => setSourceText(e.target.value)} placeholder="Scopus, Web of Science" className={input} />
          </div>
          <div>
            <span className={label}>Abstract</span>
            <textarea value={f.abstract_text ?? ""} onChange={(e) => set("abstract_text", e.target.value || null)} rows={3} className={cn(input, "resize-y")} />
          </div>
          {allowPdf && (
            <div>
              <span className={label}>PDF (optional)</span>
              <button onClick={pickPdf} className="inline-flex items-center gap-2 px-3 py-1.5 text-[12px] rounded-[var(--radius-sm)] border border-dashed border-[var(--color-paper-300)] text-[var(--color-paper-600)] hover:border-[var(--color-ink-400)] transition-paper">
                <Upload size={13} strokeWidth={1.75} /> {pdfPath ? (pdfPath.split("/").pop() ?? "PDF selected") : "Choose PDF…"}
              </button>
            </div>
          )}
        </div>

        {err && <p className="mt-3 text-[12px] text-[var(--color-error-600)]">{err}</p>}

        <div className="flex items-center justify-end gap-2 mt-4">
          <Button variant="ghost" size="sm" onClick={onCancel}>Cancel</Button>
          <Button variant="primary" size="sm" onClick={save} disabled={saving}>
            {saving ? <Loader2 size={13} className="animate-spin" /> : articleId != null ? "Save changes" : "Add to library"}
          </Button>
        </div>
      </div>
    </Card>
  );
}
