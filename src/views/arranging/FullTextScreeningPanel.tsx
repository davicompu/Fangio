import { useEffect, useMemo, useState } from "react";
import {
  Ban,
  Check,
  DownloadCloud,
  ExternalLink,
  FileText,
  Loader2,
  RotateCcw,
  Sparkles,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { api, type FulltextCandidate } from "../../lib/api";
import { Button } from "../../components/ui/Button";
import { cn } from "../../lib/utils";

type Tab = "pending" | "kept" | "excluded";

/** Full-text (eligibility) screening — PRISMA's second pass. Operates on the
 * title/abstract-included set; fetch missing PDFs, read the full text, and keep
 * or exclude. Excluding here removes the article from the corpus. */
export function FullTextScreeningPanel({ refreshKey, onChanged }: { refreshKey: number; onChanged: () => void }) {
  const [items, setItems] = useState<FulltextCandidate[]>([]);
  const [loading, setLoading] = useState(true);
  const [tab, setTab] = useState<Tab>("pending");
  const [busyId, setBusyId] = useState<number | null>(null);
  const [finding, setFinding] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [excludeFor, setExcludeFor] = useState<number | null>(null);
  const [reason, setReason] = useState("");
  const [aiBusy, setAiBusy] = useState(false);
  const [aiProgress, setAiProgress] = useState<{ done: number; total: number } | null>(null);

  const reload = async () => {
    try {
      setItems(await api.listFulltextCandidates());
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => { reload(); }, [refreshKey]);

  useEffect(() => {
    const un = listen<{ done: number; total: number }>("fulltext_screen:progress", (e) => setAiProgress(e.payload));
    return () => { un.then((f) => f()); };
  }, []);

  const screenWithAi = async () => {
    setAiBusy(true);
    setMsg("AI full-text screening…");
    setAiProgress({ done: 0, total: 0 });
    try {
      const r = await api.runAiFulltextScreening();
      setMsg(`AI screened ${r.processed}: ${r.kept} kept · ${r.excluded} excluded · ${r.uncertain} left for you${r.skipped_no_pdf ? ` · ${r.skipped_no_pdf} no PDF` : ""}`);
      await reload();
      onChanged();
    } catch (e: any) {
      const m: string = e?.message ?? String(e);
      setMsg(m.includes("reasoning model") ? "Pick a reasoning model in Settings first." : `Model error: ${m}`);
    } finally {
      setAiBusy(false);
      setAiProgress(null);
    }
  };

  const phaseOf = (i: FulltextCandidate): Tab =>
    i.fulltext_status === "included" ? "kept" : i.fulltext_status === "excluded" ? "excluded" : "pending";

  const counts = useMemo(() => {
    const c = { pending: 0, kept: 0, excluded: 0 } as Record<Tab, number>;
    for (const i of items) c[phaseOf(i)]++;
    return c;
  }, [items]);
  const missingPdf = useMemo(() => items.filter((i) => phaseOf(i) === "pending" && !i.pdf_path).length, [items]);
  const filtered = items.filter((i) => phaseOf(i) === tab);

  const findPdfs = async () => {
    setFinding(true);
    setMsg("Searching Unpaywall for open-access PDFs…");
    try {
      const r = await api.findMissingPdfs();
      const parts = [`${r.found} found`];
      if (r.no_oa > 0) parts.push(`${r.no_oa} no open-access copy`);
      if (r.no_doi > 0) parts.push(`${r.no_doi} no DOI`);
      setMsg(parts.join(" · "));
      await reload();
      onChanged();
    } catch (e: any) {
      setMsg(e?.toString() || String(e));
    } finally {
      setFinding(false);
    }
  };

  const keep = async (id: number) => {
    setBusyId(id);
    try { await api.fulltextKeepArticle(id); await reload(); onChanged(); } finally { setBusyId(null); }
  };
  const doExclude = async (id: number) => {
    setBusyId(id);
    try {
      await api.fulltextExcludeArticle(id, reason.trim() || undefined);
      setExcludeFor(null); setReason("");
      await reload(); onChanged();
    } finally { setBusyId(null); }
  };
  const reset = async (id: number) => {
    setBusyId(id);
    try { await api.resetFulltextArticle(id); await reload(); onChanged(); } finally { setBusyId(null); }
  };

  const TABS: { id: Tab; label: string }[] = [
    { id: "pending", label: "To review" },
    { id: "kept", label: "Kept" },
    { id: "excluded", label: "Excluded" },
  ];

  return (
    <div className="w-full pl-6 pr-4 xl:pl-12 xl:pr-8">
      <div className="flex items-end justify-between gap-6 mb-4">
        <div>
          <div className="eyebrow text-[var(--color-paper-500)]">Eligibility</div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">Full-text screening</h2>
          <p className="text-[13px] text-[var(--color-paper-600)] mt-1.5 max-w-xl leading-relaxed">
            Assess the full text of the articles that passed title &amp; abstract screening. Keep the
            eligible ones; exclude the rest (they leave the corpus).
          </p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <Button onClick={screenWithAi} variant="secondary" disabled={aiBusy || finding} className="whitespace-nowrap" title="AI reads the full text and suggests keep/exclude for unreviewed articles">
            {aiBusy ? <Loader2 size={14} className="animate-spin" /> : <Sparkles size={14} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />}
            {aiBusy && aiProgress && aiProgress.total > 0 ? `Screening ${aiProgress.done}/${aiProgress.total}` : "Screen with AI"}
          </Button>
          <Button onClick={findPdfs} variant="secondary" disabled={finding || aiBusy} className="whitespace-nowrap" title="Fetch open-access PDFs (Unpaywall) for articles missing one">
            {finding ? <Loader2 size={14} className="animate-spin" /> : <DownloadCloud size={14} strokeWidth={1.75} />}
            Find PDFs{missingPdf > 0 ? ` (${missingPdf})` : ""}
          </Button>
        </div>
      </div>

      {msg && (
        <div className="mb-4 px-4 py-2.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-300)] text-[12px] text-[var(--color-paper-600)]">{msg}</div>
      )}

      <div className="flex items-center gap-1 mb-4">
        {TABS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={cn(
              "px-3 py-1.5 rounded-full text-[12px] font-medium transition-paper",
              tab === t.id ? "bg-[var(--color-ink-800)] text-white" : "bg-[var(--color-paper-100)] text-[var(--color-paper-600)] hover:bg-[var(--color-paper-200)]",
            )}
          >
            {t.label} <span className="tabular-nums opacity-70">{counts[t.id]}</span>
          </button>
        ))}
      </div>

      {loading ? (
        <div className="flex items-center gap-2 text-[13px] text-[var(--color-paper-500)]"><Loader2 size={14} className="animate-spin" /> Loading…</div>
      ) : filtered.length === 0 ? (
        <p className="text-[13px] text-[var(--color-paper-500)] italic">
          {tab === "pending" ? "Nothing left to review — every included article has a full-text decision." : `No ${tab} articles.`}
        </p>
      ) : (
        <div className="space-y-3">
          {filtered.map((a) => (
            <div key={a.id} className="rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-white p-4">
              <div className="font-medium text-[14px] text-[var(--color-ink-900)] leading-snug">{a.title}</div>
              <div className="text-[12px] text-[var(--color-paper-600)] mt-0.5">
                {[a.authors, a.year?.toString()].filter(Boolean).join(" · ") || "—"}
              </div>
              {a.abstract_text && (
                <p className="text-[12px] text-[var(--color-paper-500)] leading-[1.6] mt-1.5 line-clamp-2">{a.abstract_text}</p>
              )}

              <div className="mt-3 pt-3 border-t border-[var(--color-paper-200)] flex items-center justify-between gap-3">
                {a.pdf_path ? (
                  <button
                    onClick={() => api.openPdfPath(a.pdf_path!).catch(() => {})}
                    className="inline-flex items-center gap-1.5 text-[12px] font-medium text-[var(--color-accent-700)] hover:underline underline-offset-2"
                  >
                    <FileText size={13} strokeWidth={1.75} /> Open PDF <ExternalLink size={11} strokeWidth={1.75} />
                  </button>
                ) : (
                  <span className="text-[11px] text-[var(--color-warn-600)]">No PDF — use “Find PDFs” or add one in the corpus.</span>
                )}

                <div className="flex items-center gap-2 shrink-0">
                  {tab === "pending" ? (
                    <>
                      <button
                        onClick={() => keep(a.id)}
                        disabled={busyId === a.id}
                        className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-semibold bg-[var(--color-ok-100)] border border-[var(--color-ok-300)] text-[var(--color-ok-700)] hover:bg-[var(--color-ok-200)] disabled:opacity-40 transition-paper"
                      >
                        {busyId === a.id ? <Loader2 size={12} className="animate-spin" /> : <Check size={13} strokeWidth={2.25} />} Keep
                      </button>
                      <button
                        onClick={() => setExcludeFor(excludeFor === a.id ? null : a.id)}
                        disabled={busyId === a.id}
                        className={cn(
                          "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium border transition-paper disabled:opacity-40",
                          excludeFor === a.id
                            ? "bg-[var(--color-warn-50)] border-[var(--color-warn-400)] text-[var(--color-warn-700)]"
                            : "bg-white border-[var(--color-paper-300)] text-[var(--color-paper-600)] hover:border-[var(--color-warn-400)] hover:text-[var(--color-warn-700)]",
                        )}
                      >
                        <Ban size={13} strokeWidth={2} /> Exclude
                      </button>
                    </>
                  ) : (
                    <>
                      <span className={cn(
                        "inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-semibold uppercase tracking-wide",
                        tab === "kept" ? "bg-[var(--color-ok-100)] text-[var(--color-ok-700)]" : "bg-[var(--color-warn-100)] text-[var(--color-warn-700)]",
                      )}>
                        {tab === "kept" ? <Check size={11} strokeWidth={2.5} /> : <Ban size={11} strokeWidth={2.5} />}
                        {tab === "kept" ? "Kept" : "Excluded"}
                      </span>
                      <button
                        onClick={() => reset(a.id)}
                        disabled={busyId === a.id}
                        className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] hover:bg-[var(--color-paper-100)] disabled:opacity-40 transition-paper"
                      >
                        {busyId === a.id ? <Loader2 size={12} className="animate-spin" /> : <RotateCcw size={12} strokeWidth={2} />} Undo
                      </button>
                    </>
                  )}
                </div>
              </div>

              {excludeFor === a.id && (
                <div className="mt-3 flex items-center gap-2">
                  <input
                    type="text"
                    value={reason}
                    onChange={(e) => setReason(e.target.value)}
                    onKeyDown={(e) => { if (e.key === "Enter") doExclude(a.id); }}
                    placeholder="Reason (optional) — e.g. not empirical, wrong population"
                    className="flex-1 px-3 py-1.5 text-[12px] rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] bg-white focus:outline-none focus:ring-1 focus:ring-[var(--color-warn-400)]"
                  />
                  <button
                    onClick={() => doExclude(a.id)}
                    disabled={busyId === a.id}
                    className="shrink-0 inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-semibold bg-[var(--color-warn-600)] text-white hover:bg-[var(--color-warn-700)] disabled:opacity-40 transition-paper"
                  >
                    Exclude at full text
                  </button>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
