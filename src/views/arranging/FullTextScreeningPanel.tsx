import { useEffect, useMemo, useState } from "react";
import {
  Ban,
  Check,
  DownloadCloud,
  ExternalLink,
  FileText,
  Loader2,
  RotateCcw,
} from "lucide-react";
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

  const reload = async () => {
    try {
      setItems(await api.listFulltextCandidates());
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => { reload(); }, [refreshKey]);

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
    <div className="w-full pl-6 pr-4 xl:pl-12 xl:pr-8 max-w-4xl">
      <div className="flex items-end justify-between mb-4">
        <div>
          <div className="eyebrow text-[var(--color-paper-500)]">Eligibility</div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">Full-text screening</h2>
          <p className="text-[13px] text-[var(--color-paper-600)] mt-1.5 max-w-xl leading-relaxed">
            Assess the full text of the articles that passed title &amp; abstract screening. Keep the
            eligible ones; exclude the rest (they leave the corpus).
          </p>
        </div>
        <Button onClick={findPdfs} variant="secondary" disabled={finding} title="Fetch open-access PDFs (Unpaywall) for articles missing one">
          {finding ? <Loader2 size={14} className="animate-spin" /> : <DownloadCloud size={14} strokeWidth={1.75} />}
          Find PDFs{missingPdf > 0 ? ` (${missingPdf})` : ""}
        </Button>
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
              <div className="flex items-start gap-3">
                <div className="flex-1 min-w-0">
                  <div className="font-medium text-[14px] text-[var(--color-ink-900)] leading-snug">{a.title}</div>
                  <div className="text-[12px] text-[var(--color-paper-600)] mt-0.5">
                    {[a.authors, a.year?.toString()].filter(Boolean).join(" · ") || "—"}
                  </div>
                  {a.abstract_text && (
                    <p className="text-[12px] text-[var(--color-paper-500)] leading-[1.6] mt-1.5 line-clamp-3">{a.abstract_text}</p>
                  )}
                  <div className="mt-2">
                    {a.pdf_path ? (
                      <button
                        onClick={() => api.openPdfPath(a.pdf_path!).catch(() => {})}
                        className="inline-flex items-center gap-1.5 text-[12px] font-medium text-[var(--color-accent-700)] hover:underline underline-offset-2"
                      >
                        <FileText size={12} strokeWidth={1.75} /> Open PDF <ExternalLink size={11} strokeWidth={1.75} />
                      </button>
                    ) : (
                      <span className="text-[11px] text-[var(--color-warn-600)]">No PDF attached — use “Find PDFs” or add one in the corpus.</span>
                    )}
                  </div>
                </div>
                <div className="flex flex-col items-end gap-1.5 shrink-0">
                  {tab === "pending" ? (
                    <>
                      <Button variant="secondary" size="sm" onClick={() => keep(a.id)} disabled={busyId === a.id}>
                        {busyId === a.id ? <Loader2 size={12} className="animate-spin" /> : <Check size={12} strokeWidth={2} />} Keep
                      </Button>
                      <Button variant="ghost" size="sm" onClick={() => setExcludeFor(excludeFor === a.id ? null : a.id)} disabled={busyId === a.id}>
                        <Ban size={12} strokeWidth={2} /> Exclude
                      </Button>
                    </>
                  ) : (
                    <Button variant="ghost" size="sm" onClick={() => reset(a.id)} disabled={busyId === a.id}>
                      {busyId === a.id ? <Loader2 size={12} className="animate-spin" /> : <RotateCcw size={12} strokeWidth={2} />} Undo
                    </Button>
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
                  <Button variant="primary" size="sm" onClick={() => doExclude(a.id)} disabled={busyId === a.id}>
                    Exclude at full text
                  </Button>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
