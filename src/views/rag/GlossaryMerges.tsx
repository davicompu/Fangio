import { useCallback, useEffect, useRef, useState } from "react";
import { ArrowLeftRight, Check, ChevronDown, ChevronRight, Loader2, RotateCcw, Sparkles, X } from "lucide-react";
import { api, type GlossaryMergeOverview, type MergeSuggestion } from "../../lib/api";
import { cn } from "../../lib/utils";

const DEFAULT_THRESHOLD = 0.88;

/** Suggested glossary merges by meaning. Nothing changes the glossary, the
 *  concept map or the overlap counts until the researcher accepts a merge. */
export function GlossaryMerges({ termCount, onChanged }: { termCount: number; onChanged: () => Promise<void> | void }) {
  const [open, setOpen] = useState(false);
  const [threshold, setThreshold] = useState(DEFAULT_THRESHOLD);
  const [data, setData] = useState<GlossaryMergeOverview | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  // Which side to keep per suggestion; default = the term in more papers.
  const [keepB, setKeepB] = useState<Record<string, boolean>>({});

  useEffect(() => {
    api.getSettings()
      .then((s) => {
        const t = s["glossary.merge_threshold"];
        if (typeof t === "number") setThreshold(t);
      })
      .catch(() => {});
  }, []);

  // Only the latest request may update the panel (the slider fires often).
  const seq = useRef(0);
  const load = useCallback(async (t: number) => {
    const mine = ++seq.current;
    setLoading(true);
    setError(null);
    try {
      const d = await api.glossaryMergeSuggestions(t);
      if (mine === seq.current) setData(d);
    } catch (e) {
      if (mine !== seq.current) return;
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg.includes("Connection refused") || msg.includes("error sending request")
        ? "LM Studio isn't reachable, so merge suggestions can't be computed."
        : msg);
    } finally {
      setLoading(false);
    }
  }, []);

  // Recompute when opened, when the threshold settles, or when terms change.
  useEffect(() => {
    if (!open || termCount < 2) return;
    const timer = setTimeout(() => void load(threshold), 300);
    return () => clearTimeout(timer);
  }, [open, threshold, termCount, load]);

  const key = (s: MergeSuggestion) => `${s.a.id}-${s.b.id}`;
  const keepsB = (s: MergeSuggestion) => keepB[key(s)] ?? s.b.article_count > s.a.article_count;

  const act = async (id: string, fn: () => Promise<unknown>) => {
    setBusy(id);
    setError(null);
    try {
      await fn();
      await onChanged();
      // Dismissing doesn't change the term count, so reload explicitly; the
      // sequence guard makes the extra reload after a merge harmless.
      await load(threshold);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  };

  const suggestions = data?.suggestions ?? [];
  const accepted = (data?.decisions ?? []).filter((d) => d.status === "accepted");
  const declined = (data?.decisions ?? []).filter((d) => d.status !== "accepted");

  if (termCount < 2) return null;

  return (
    <div className="border-b border-[var(--color-paper-200)] bg-white shrink-0">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center gap-2 px-5 py-2 text-[12px] font-medium text-[var(--color-ink-800)] hover:bg-[var(--color-paper-50)]"
      >
        {open ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
        <Sparkles size={12} strokeWidth={2} className="text-[var(--color-accent-500)]" />
        Suggested merges
        {data && open && <span className="text-[var(--color-paper-500)] font-normal">· {suggestions.length} to review</span>}
      </button>

      {open && (
        <div className="px-5 pb-3 space-y-3 max-h-[45vh] overflow-y-auto">
          <div className="flex items-center gap-3 text-[11px] text-[var(--color-paper-600)]">
            <label className="flex items-center gap-2">
              Suggest pairs at least
              <input
                type="range"
                min={0.7}
                max={0.99}
                step={0.01}
                value={threshold}
                onChange={(e) => setThreshold(Number(e.target.value))}
                aria-label="Similarity threshold for merge suggestions"
              />
              <span className="tabular-nums w-8">{threshold.toFixed(2)}</span>
              similar
            </label>
            {threshold !== DEFAULT_THRESHOLD && (
              <button type="button" onClick={() => setThreshold(DEFAULT_THRESHOLD)} className="underline-offset-2 hover:underline">
                reset ({DEFAULT_THRESHOLD})
              </button>
            )}
            {loading && <Loader2 size={11} className="animate-spin" />}
          </div>
          <p className="text-[11px] text-[var(--color-paper-500)] leading-snug">
            Compares the meaning of term names with your embedding model{data ? ` (${data.embedding_model})` : ""}.
            Similar wording isn't always the same concept, e.g. "ChatGPT-assisted" vs "Non-ChatGPT-assisted", so check
            each pair. Nothing changes until you merge.
          </p>
          {error && <p className="text-[11px] text-[var(--color-warn-700)]">{error}</p>}

          {data && suggestions.length === 0 && !loading && (
            <p className="text-[11px] italic text-[var(--color-paper-500)]">No pairs to review at this threshold.</p>
          )}
          {suggestions.map((s) => {
            const k = key(s);
            const kb = keepsB(s);
            const keep = kb ? s.b : s.a;
            const other = kb ? s.a : s.b;
            return (
              <div key={k} className="flex items-center gap-2 text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-paper-200)] px-3 py-2">
                <span className="font-mono text-[10px] text-[var(--color-accent-700)] tabular-nums w-8">{s.similarity.toFixed(2)}</span>
                <span className="flex-1 min-w-0">
                  <span className="font-semibold text-[var(--color-ink-900)]">{keep.term}</span>
                  <span className="text-[var(--color-paper-400)]"> ({keep.article_count}) </span>
                  <span className="text-[var(--color-paper-500)]">absorbs </span>
                  <span className="text-[var(--color-ink-800)]">{other.term}</span>
                  <span className="text-[var(--color-paper-400)]"> ({other.article_count})</span>
                </span>
                <button
                  type="button"
                  title="Swap which name is kept"
                  onClick={() => setKeepB((m) => ({ ...m, [k]: !kb }))}
                  className="p-1 rounded-[var(--radius-xs)] text-[var(--color-paper-500)] hover:bg-[var(--color-paper-100)]"
                >
                  <ArrowLeftRight size={12} />
                </button>
                <button
                  type="button"
                  disabled={busy !== null}
                  onClick={() => act(k, () => api.acceptGlossaryMerge(keep.id, other.id, s.similarity, data!.embedding_model))}
                  className="inline-flex items-center gap-1 px-2 py-1 rounded-[var(--radius-xs)] bg-[var(--color-ink-800)] text-white text-[11px] font-medium disabled:opacity-50"
                >
                  {busy === k ? <Loader2 size={11} className="animate-spin" /> : <Check size={11} />} Merge
                </button>
                <button
                  type="button"
                  disabled={busy !== null}
                  onClick={() => act(k, () => api.dismissGlossaryMerge(s.a.id, s.b.id, s.similarity, data!.embedding_model))}
                  className="inline-flex items-center gap-1 px-2 py-1 rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] text-[11px] text-[var(--color-paper-600)] disabled:opacity-50"
                >
                  <X size={11} /> Not the same
                </button>
              </div>
            );
          })}

          {accepted.length > 0 && (
            <div>
              <p className="eyebrow text-[var(--color-paper-500)] mb-1">Merged</p>
              {accepted.map((d) => {
                const other = d.keep_term === d.term_a ? d.term_b : d.term_a;
                return (
                  <div key={d.id} className="flex items-center gap-2 text-[12px] py-1">
                    <span className="flex-1 min-w-0 text-[var(--color-ink-800)]">
                      <span className="font-semibold">{d.keep_term}</span>
                      <span className="text-[var(--color-paper-500)]">{d.in_effect ? " now includes " : " + "}</span>
                      {other}
                      {!d.in_effect && (
                        <span className="block text-[11px] text-[var(--color-paper-500)]">
                          Not in effect now: “{d.keep_term}” isn't in the glossary after the last extraction. It's applied again automatically if a later extraction finds it.
                        </span>
                      )}
                    </span>
                    {d.in_effect && <button
                      type="button"
                      disabled={busy !== null}
                      onClick={() => act(`u${d.id}`, () => api.undoGlossaryMerge(d.id))}
                      className="inline-flex items-center gap-1 text-[11px] text-[var(--color-accent-700)] hover:underline disabled:opacity-50"
                    >
                      <RotateCcw size={11} /> Undo
                    </button>}
                  </div>
                );
              })}
            </div>
          )}

          {declined.length > 0 && (
            <div>
              <p className="eyebrow text-[var(--color-paper-500)] mb-1">Kept separate</p>
              {declined.map((d) => (
                <div key={d.id} className="flex items-center gap-2 text-[12px] py-1">
                  <span className={cn("flex-1 min-w-0 text-[var(--color-paper-600)]")}>
                    {d.term_a} / {d.term_b}
                    <span className="text-[var(--color-paper-400)]"> · {d.status === "undone" ? "merge undone" : "not the same"}</span>
                  </span>
                  <button
                    type="button"
                    disabled={busy !== null}
                    onClick={() => act(`r${d.id}`, () => api.reconsiderGlossaryMerge(d.id))}
                    className="text-[11px] text-[var(--color-paper-500)] hover:underline disabled:opacity-50"
                  >
                    Suggest again
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
