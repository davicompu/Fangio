import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
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
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { GraduationCap, Upload } from "lucide-react";
import { api, type ExclusionCriterionRow, type FulltextCandidate } from "../../lib/api";
import { Button } from "../../components/ui/Button";
import { cn } from "../../lib/utils";

type Tab = "pending" | "kept" | "excluded";

// Match the title & abstract screening page size.
const PAGE_SIZE = 5;

type FulltextResult = { processed: number; kept: number; excluded: number; uncertain: number; skipped_no_pdf: number };
type FulltextProgress = { done: number; total: number; title?: string };

export interface AiFulltextHook {
  step: "idle" | "running" | "done";
  progress: FulltextProgress | null;
  result: FulltextResult | null;
  error: string | null;
  isPaused: boolean;
  start: () => Promise<void>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  cancel: () => Promise<void>;
  reset: () => void;
}

/** Background-capable full-text AI screening, owned by ArrangingStage so the run
 *  (and its banner / pause-cancel controls) survive navigating away from the tab —
 *  mirroring the title/abstract `useAiScreening` hook. */
export function useAiFulltextScreening(): AiFulltextHook {
  const [step, setStep] = useState<AiFulltextHook["step"]>("idle");
  const [progress, setProgress] = useState<FulltextProgress | null>(null);
  const [result, setResult] = useState<FulltextResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isPaused, setIsPaused] = useState(false);
  const unlistenRef = useRef<(() => void) | null>(null);

  const start = useCallback(async () => {
    setStep("running");
    setError(null);
    setProgress(null);
    setResult(null);
    setIsPaused(false);
    try {
      unlistenRef.current = await listen<FulltextProgress>("fulltext_screen:progress", (e) => setProgress(e.payload));
      const res = await api.runAiFulltextScreening();
      setResult(res);
      setStep("done");
    } catch (e: any) {
      const msg: string = e?.message ?? String(e);
      setError(msg.includes("reasoning model") ? "Pick a reasoning model in Settings first." : msg);
      setStep("running");
    } finally {
      unlistenRef.current?.();
      unlistenRef.current = null;
    }
  }, []);

  const pause = useCallback(async () => { await api.pauseAiFulltextScreening(); setIsPaused(true); }, []);
  const resume = useCallback(async () => { await api.resumeAiFulltextScreening(); setIsPaused(false); }, []);
  const cancel = useCallback(async () => { await api.cancelAiFulltextScreening(); setIsPaused(false); }, []);
  const reset = useCallback(() => { setStep("idle"); setProgress(null); setResult(null); setError(null); setIsPaused(false); }, []);

  return { step, progress, result, error, isPaused, start, pause, resume, cancel, reset };
}

/** Full-text (eligibility) screening — PRISMA's second pass. Operates on the
 * title/abstract-included set; fetch missing PDFs, read the full text, and keep
 * or exclude. Excluding here removes the article from the corpus. */
export function FullTextScreeningPanel({ refreshKey, onChanged, screening }: { refreshKey: number; onChanged: () => void; screening: AiFulltextHook }) {
  const [items, setItems] = useState<FulltextCandidate[]>([]);
  const [loading, setLoading] = useState(true);
  const [tab, setTab] = useState<Tab>("pending");
  const [busyId, setBusyId] = useState<number | null>(null);
  const [finding, setFinding] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [criteria, setCriteria] = useState<ExclusionCriterionRow[]>([]);
  const [findProgress, setFindProgress] = useState<{ done: number; total: number; title?: string } | null>(null);
  const [page, setPage] = useState(0);

  // The AI run is owned by the parent hook so it keeps going (with a banner +
  // pause/cancel) when you leave the tab. Mirror its state locally.
  const aiBusy = screening.step === "running";
  const aiProgress = screening.progress;

  const reload = async () => {
    try {
      const [list, crits] = await Promise.all([api.listFulltextCandidates(), api.listCriteria()]);
      setItems(list);
      // EC1 (Duplicate) is handled at deduplication, not screening — omit it.
      setCriteria(crits.filter((c) => c.number > 1));
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => { reload(); }, [refreshKey]);

  // Find-PDFs progress is still local (full-text AI progress comes via the hook).
  useEffect(() => {
    const b = listen<{ done: number; total: number; title?: string }>("find_pdfs:progress", (e) => setFindProgress(e.payload));
    return () => { b.then((f) => f()); };
  }, []);

  // Native file-drop → attach a PDF to whichever card it's dropped on. Tauri gives
  // a window-level drop with the physical cursor position; we hit-test the card
  // rects to route the file to the right article (the list shows several at once).
  const cardRefs = useRef<Map<number, HTMLDivElement>>(new Map());
  const [dragOverId, setDragOverId] = useState<number | null>(null);
  const articleAtPoint = (x: number, y: number): number | null => {
    const dpr = window.devicePixelRatio || 1;
    const cx = x / dpr, cy = y / dpr;
    for (const [id, el] of cardRefs.current) {
      const r = el.getBoundingClientRect();
      if (cx >= r.left && cx <= r.right && cy >= r.top && cy <= r.bottom) return id;
    }
    return null;
  };
  useEffect(() => {
    let unDrop: (() => void) | null = null, unOver: (() => void) | null = null, unLeave: (() => void) | null = null;
    listen<{ paths: string[]; position: { x: number; y: number } }>("tauri://drag-drop", async (e) => {
      setDragOverId(null);
      const pdf = e.payload.paths.find((p) => p.toLowerCase().endsWith(".pdf"));
      if (!pdf) return;
      const id = articleAtPoint(e.payload.position.x, e.payload.position.y);
      if (id == null) return;
      setBusyId(id);
      setMsg(null);
      try { await api.dropPdfForArticle(pdf, id); await reload(); onChanged(); }
      catch (err: any) { setMsg(err?.message ?? String(err)); }
      finally { setBusyId(null); }
    }).then((f) => { unDrop = f; });
    listen<{ position: { x: number; y: number } }>("tauri://drag-over", (e) => {
      setDragOverId(articleAtPoint(e.payload.position.x, e.payload.position.y));
    }).then((f) => { unOver = f; });
    listen("tauri://drag-leave", () => setDragOverId(null)).then((f) => { unLeave = f; });
    return () => { unDrop?.(); unOver?.(); unLeave?.(); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // When the background AI run finishes, refresh the list and show a summary.
  useEffect(() => {
    if (screening.step === "done" && screening.result) {
      const r = screening.result;
      setMsg(`AI screened ${r.processed}: ${r.kept} kept · ${r.excluded} excluded · ${r.uncertain} left for you${r.skipped_no_pdf ? ` · ${r.skipped_no_pdf} no PDF` : ""}`);
      reload();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [screening.step, screening.result]);

  const screenWithAi = async () => {
    setMsg(null);
    await screening.start();
  };

  // Tabs mirror title/abstract and are driven by the article's corpus status:
  // Included = currently in the corpus (passed title/abstract), Excluded = removed
  // at any phase, Pending = still pending title/abstract. Full-text keep/exclude
  // then refines the Included set.
  const phaseOf = (i: FulltextCandidate): Tab =>
    i.ta_status === "excluded"
      ? "excluded"
      : i.ta_status === "retrieved"
      ? "pending"
      : "kept"; // ta_status === "included"

  // Where an excluded article left the corpus, plus the AI logic if any.
  const exclusionNote = (i: FulltextCandidate): string =>
    i.fulltext_status === "excluded"
      ? "Excluded at full-text screening"
      : i.ai_rationale
      ? "Excluded during purification (AI judgment)"
      : i.exclusion_criterion === 2
      ? "Excluded during purification (search period)"
      : "Excluded during purification / title & abstract";

  const counts = useMemo(() => {
    const c = { pending: 0, kept: 0, excluded: 0 } as Record<Tab, number>;
    for (const i of items) c[phaseOf(i)]++;
    return c;
  }, [items]);
  // The article status backing the active tab — "Find PDFs" fetches for this set only.
  const tabStatus = tab === "kept" ? "included" : tab === "pending" ? "retrieved" : "excluded";
  // Articles in the active tab missing a PDF — the set "Find PDFs" will fetch for.
  const missingPdf = useMemo(
    () => items.filter((i) => i.ta_status === tabStatus && !i.pdf_path).length,
    [items, tabStatus],
  );
  const filtered = items.filter((i) => phaseOf(i) === tab);
  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const currentPage = Math.min(page, totalPages - 1);
  const pageItems = filtered.slice(currentPage * PAGE_SIZE, currentPage * PAGE_SIZE + PAGE_SIZE);

  // Reset to the first page when the tab changes or the list shrinks past the page.
  useEffect(() => { setPage(0); }, [tab]);
  useEffect(() => { if (page > totalPages - 1) setPage(0); }, [totalPages, page]);

  const findPdfs = async () => {
    setFinding(true);
    setFindProgress({ done: 0, total: 0 });
    setMsg(null);
    try {
      const r = await api.findMissingPdfs([tabStatus]);
      const parts = [`${r.found} PDF${r.found !== 1 ? "s" : ""} found`];
      if (r.no_oa > 0) parts.push(`${r.no_oa} no open-access copy`);
      if (r.no_doi > 0) parts.push(`${r.no_doi} no DOI`);
      setMsg(parts.join(" · "));
      await reload();
      onChanged();
    } catch (e: any) {
      setMsg(e?.toString() || String(e));
    } finally {
      setFinding(false);
      setFindProgress(null);
    }
  };

  const keep = async (id: number) => {
    setBusyId(id);
    try { await api.fulltextKeepArticle(id); await reload(); onChanged(); } finally { setBusyId(null); }
  };
  const doExclude = async (id: number, criterionNumber?: number) => {
    setBusyId(id);
    try {
      await api.fulltextExcludeArticle(id, criterionNumber);
      await reload(); onChanged();
    } finally { setBusyId(null); }
  };
  const reset = async (id: number) => {
    setBusyId(id);
    try { await api.resetFulltextArticle(id); await reload(); onChanged(); } finally { setBusyId(null); }
  };
  // Revert a decision all the way back to Pending Screening (status='retrieved').
  const revert = async (id: number) => {
    setBusyId(id);
    try { await api.unscreenArticle(id); await reload(); onChanged(); } finally { setBusyId(null); }
  };

  // Manually attach a PDF the researcher located themselves.
  const addPdf = async (id: number) => {
    const sel = await openDialog({ multiple: false, filters: [{ name: "PDF", extensions: ["pdf"] }], title: "Select a PDF for this article" });
    if (!sel || Array.isArray(sel)) return;
    setBusyId(id);
    try { await api.dropPdfForArticle(sel as string, id); await reload(); onChanged(); }
    catch (e: any) { setMsg(e?.message ?? String(e)); }
    finally { setBusyId(null); }
  };
  // Open DOI (or a Scholar search) so the researcher can find the PDF online.
  const findOnline = (a: FulltextCandidate) => {
    const url = a.doi ? `https://doi.org/${a.doi}` : `https://scholar.google.com/scholar?q=${encodeURIComponent(a.title)}`;
    openUrl(url).catch(() => {});
  };

  const TABS: { id: Tab; label: string }[] = [
    { id: "pending", label: "Pending Screening" },
    { id: "kept", label: "Included" },
    { id: "excluded", label: "Excluded" },
  ];

  const renderItem = (a: FulltextCandidate, idx: number) => {
    const ftKept = a.fulltext_status === "included";
    const ftExcluded = a.fulltext_status === "excluded";
    const earlierExcluded = a.ta_status === "excluded" && !ftExcluded;
    // Articles still in the corpus without a full-text decision can be reviewed
    // (kept = confirmed eligible / pending promoted; excluded = removed here).
    const canReview = !ftKept && !ftExcluded && !earlierExcluded;
    const number = currentPage * PAGE_SIZE + idx + 1;
    return (
    <div
      key={a.id}
      ref={(el) => { if (el) cardRefs.current.set(a.id, el); else cardRefs.current.delete(a.id); }}
      className={cn(
        "rounded-[var(--radius-md)] border bg-white p-4 transition-paper",
        dragOverId === a.id && !a.pdf_path
          ? "border-[var(--color-accent-400)] ring-2 ring-[var(--color-accent-400)]/30 bg-[var(--color-accent-50)]"
          : "border-[var(--color-paper-300)]",
      )}
    >
      <div className="flex items-start gap-4">
        <div className="shrink-0 flex flex-col items-center gap-1 mt-0.5">
          <div className="w-7 h-7 rounded-full flex items-center justify-center text-[12px] font-semibold tabular-nums border bg-[var(--color-paper-100)] border-[var(--color-paper-300)] text-[var(--color-ink-700)]" title={`Article ${number} of ${filtered.length}`}>
            {number}
          </div>
          <div className="font-mono text-[9px] text-[var(--color-paper-400)] tabular-nums">#{a.id}</div>
        </div>

        {/* Left: article + decision */}
        <div className="flex-1 min-w-0">
          <div className="font-medium text-[14px] text-[var(--color-ink-900)] leading-snug">{a.title}</div>
          <div className="text-[12px] text-[var(--color-paper-600)] mt-0.5">
            {[a.authors, a.year?.toString()].filter(Boolean).join(" · ") || "—"}
          </div>
          {(a.journal || a.doi) && (
            <div className="mt-1 flex items-center gap-2 flex-wrap text-[11px]">
              {a.journal && <span className="italic text-[var(--color-paper-500)]">{a.journal}</span>}
              {a.doi && (
                <span className="font-mono text-[var(--color-paper-400)]">{a.journal ? "· " : ""}{a.doi}</span>
              )}
            </div>
          )}
          {a.abstract_text && (
            <p className="text-[12px] text-[var(--color-paper-500)] leading-[1.6] mt-1.5 line-clamp-2">{a.abstract_text}</p>
          )}

          {/* Exclusion provenance — where it left the corpus + the AI logic, if any. */}
          {tab === "excluded" && (
            <div className="mt-2">
              <div className="inline-flex items-center gap-1.5 text-[11px] font-medium text-[var(--color-warn-700)]">
                <Ban size={11} strokeWidth={2} /> {exclusionNote(a)}
                {a.exclusion_criterion != null && (
                  <span className="text-[var(--color-paper-400)] font-normal">· EC{a.exclusion_criterion}</span>
                )}
              </div>
              {(a.ai_rationale || a.exclusion_rationale) && (
                <p className="mt-1.5 text-[11px] text-[var(--color-paper-600)] leading-[1.55] bg-[var(--color-paper-100)] border border-[var(--color-paper-200)] rounded-[var(--radius-sm)] px-2.5 py-1.5">
                  <span className="font-semibold text-[var(--color-ink-700)]">{a.ai_rationale ? "AI logic: " : "Reason: "}</span>
                  {a.ai_rationale || a.exclusion_rationale}
                </p>
              )}
            </div>
          )}

          {/* Decision row — Keep + inline exclusion criteria (incl. custom), like title & abstract */}
          <div className="mt-3 pt-3 border-t border-[var(--color-paper-200)]">
        {canReview ? (
          <div className="flex items-center gap-2 flex-wrap">
            <button
              onClick={() => keep(a.id)}
              disabled={busyId === a.id}
              title="Confirm eligible at full text (enters the corpus for coding)"
              className={cn(
                "inline-flex items-center gap-1.5 px-3.5 py-2 rounded-[var(--radius-sm)] text-[12px] font-semibold transition-paper",
                "border border-[var(--color-ok-300)] bg-[var(--color-ok-50)] text-[var(--color-ok-700)]",
                "hover:bg-[var(--color-ok-500)] hover:text-white hover:border-[var(--color-ok-500)]",
                "disabled:opacity-50 disabled:cursor-not-allowed",
              )}
            >
              {busyId === a.id ? <Loader2 size={12} className="animate-spin" /> : <Check size={13} strokeWidth={2.25} />} Keep
            </button>
            <span className="text-[11px] text-[var(--color-paper-500)] mx-1">or exclude with…</span>
            {criteria.map((c) => (
              <button
                key={c.id}
                onClick={() => doExclude(a.id, c.number)}
                disabled={busyId === a.id}
                title={c.rationale ?? c.label}
                className={cn(
                  "px-2.5 py-1 text-[11px] rounded-full border transition-paper",
                  "border-[var(--color-warn-300)] bg-[var(--color-warn-50)] text-[var(--color-warn-700)]",
                  "hover:bg-[var(--color-warn-500)] hover:text-white hover:border-[var(--color-warn-500)]",
                  "disabled:opacity-50 disabled:cursor-not-allowed",
                )}
              >
                #{c.number} {c.label}
              </button>
            ))}
            {a.ta_status === "included" && (
              <button
                onClick={() => revert(a.id)}
                disabled={busyId === a.id}
                title="Send back to pending screening"
                className="ml-1 inline-flex items-center gap-1 text-[11px] font-medium text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] disabled:opacity-40"
              >
                <RotateCcw size={11} strokeWidth={2} /> Revert
              </button>
            )}
          </div>
        ) : (
          <div className="flex items-center gap-2 flex-wrap">
            <span className={cn(
              "inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-semibold uppercase tracking-wide",
              ftKept ? "bg-[var(--color-ok-100)] text-[var(--color-ok-700)]" : "bg-[var(--color-warn-100)] text-[var(--color-warn-700)]",
            )}>
              {ftKept ? <Check size={11} strokeWidth={2.5} /> : <Ban size={11} strokeWidth={2.5} />}
              {ftKept ? "Reviewed · kept" : "Excluded"}
            </span>
            {(ftKept || ftExcluded) ? (
              <button
                onClick={() => reset(a.id)}
                disabled={busyId === a.id}
                className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] hover:bg-[var(--color-paper-100)] disabled:opacity-40 transition-paper"
              >
                {busyId === a.id ? <Loader2 size={12} className="animate-spin" /> : <RotateCcw size={12} strokeWidth={2} />} Undo
              </button>
            ) : (
              <button
                onClick={() => revert(a.id)}
                disabled={busyId === a.id}
                title="Reverse the exclusion and send back to pending screening"
                className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] hover:bg-[var(--color-paper-100)] disabled:opacity-40 transition-paper"
              >
                {busyId === a.id ? <Loader2 size={12} className="animate-spin" /> : <RotateCcw size={12} strokeWidth={2} />} Revert
              </button>
            )}
          </div>
        )}
          </div>{/* decision row */}
        </div>{/* left content column */}

        {/* Right: PDF zone — drop/click to attach (or open), like the coding panel. */}
        <div className="shrink-0 w-[150px] xl:w-[160px] flex flex-col gap-2">
          {a.pdf_path ? (
            <button
              onClick={() => api.openPdfPath(a.pdf_path!).catch(() => {})}
              className="w-full flex flex-col items-center justify-center gap-1.5 px-3 py-4 rounded-[var(--radius-sm)] bg-[var(--color-ok-50)] border border-[var(--color-ok-200)] hover:bg-[var(--color-ok-100)] text-[var(--color-ok-700)] transition-paper"
            >
              <FileText size={18} strokeWidth={1.5} />
              <span className="text-[12px] font-medium">Open PDF</span>
            </button>
          ) : (
            <button
              onClick={() => addPdf(a.id)}
              disabled={busyId === a.id}
              className={cn(
                "w-full flex flex-col items-center justify-center gap-1 px-3 py-4 rounded-[var(--radius-sm)] border border-dashed text-center transition-paper disabled:opacity-50",
                dragOverId === a.id
                  ? "border-[var(--color-ink-500)] bg-[var(--color-paper-100)] text-[var(--color-ink-700)]"
                  : "border-[var(--color-paper-300)] text-[var(--color-paper-400)] hover:border-[var(--color-paper-500)] hover:text-[var(--color-paper-600)] hover:bg-[var(--color-paper-50)]",
              )}
              title="Drop a PDF here, or click to choose one"
            >
              {busyId === a.id ? <Loader2 size={18} className="animate-spin" /> : <Upload size={18} strokeWidth={1.5} />}
              <span className="text-[11px] font-medium leading-snug">{dragOverId === a.id ? "Release to attach" : "Drop or click"}</span>
              <span className="text-[10px] leading-snug">to attach a PDF</span>
              <span className="text-[9px] font-mono text-[var(--color-paper-400)] leading-snug">or: id{a.id}_file.pdf</span>
            </button>
          )}
          <button
            onClick={() => findOnline(a)}
            className="inline-flex items-center justify-center gap-1 text-[11px] font-medium text-[var(--color-accent-600)] hover:text-[var(--color-accent-700)]"
            title={a.doi ? "Open the DOI page" : "Search Google Scholar"}
          >
            {a.doi ? <ExternalLink size={11} strokeWidth={1.75} /> : <GraduationCap size={11} strokeWidth={1.75} />}
            {a.doi ? "Open DOI" : "Find online"}
          </button>
        </div>
      </div>{/* flex row */}
    </div>
    );
  };

  return (
    <div className="w-full pl-6 pr-4 xl:pl-12 xl:pr-8">
      <div className="flex items-end justify-between gap-6 mb-4">
        <div>
          <div className="eyebrow text-[var(--color-paper-500)]">Eligibility</div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">Full-text screening</h2>
          <p className="text-[13px] text-[var(--color-paper-600)] mt-1.5 max-w-xl leading-relaxed">
            Assess the full text of articles that passed title &amp; abstract screening — plus any still
            pending it. Keep the eligible ones; exclude the rest (they leave the corpus).
          </p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {tab === "pending" && (
            <Button onClick={screenWithAi} variant="secondary" disabled={aiBusy || finding} className="whitespace-nowrap" title="AI reads the full text and suggests keep/exclude for unreviewed articles">
              {aiBusy ? <Loader2 size={14} className="animate-spin" /> : <Sparkles size={14} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />}
              {aiBusy ? "Screening…" : "Screen with AI"}
            </Button>
          )}
          <Button onClick={findPdfs} variant="secondary" disabled={finding || aiBusy} className="whitespace-nowrap" title={`Fetch open-access PDFs (Unpaywall) for ${tab === "kept" ? "included" : tab === "excluded" ? "excluded" : "pending"} articles missing one`}>
            {finding ? <Loader2 size={14} className="animate-spin" /> : <DownloadCloud size={14} strokeWidth={1.75} />}
            Find PDFs{missingPdf > 0 ? ` (${missingPdf})` : ""}
          </Button>
        </div>
      </div>

      {(aiBusy || finding) && (() => {
        const p = aiBusy ? aiProgress : findProgress;
        const label = aiBusy ? "AI full-text screening" : "Fetching open-access PDFs";
        const pct = p && p.total > 0 ? Math.round((p.done / p.total) * 100) : 0;
        return (
          <div className="mb-4 rounded-[var(--radius-md)] border border-[var(--color-accent-200)] bg-[var(--color-accent-50)] px-4 py-3">
            <div className="flex items-center justify-between text-[12px]">
              <span className="font-semibold text-[var(--color-accent-700)] inline-flex items-center gap-2">
                <Loader2 size={13} className="animate-spin" /> {label}
              </span>
              <span className="tabular-nums text-[var(--color-accent-600)]">
                {p && p.total > 0 ? `${p.done} / ${p.total}` : "starting…"}
              </span>
            </div>
            <div className="mt-2 h-1.5 rounded-full bg-[var(--color-accent-100)] overflow-hidden">
              <div className="h-full rounded-full bg-[var(--color-accent-500)] transition-all duration-300" style={{ width: `${pct}%` }} />
            </div>
            {p?.title && <p className="mt-1.5 text-[11px] text-[var(--color-accent-700)]/70 truncate">{p.title}</p>}
          </div>
        );
      })()}

      {msg && !aiBusy && !finding && (
        <div className="mb-4 px-4 py-2.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-300)] text-[12px] text-[var(--color-paper-600)]">{msg}</div>
      )}

      <div className="flex items-center gap-1 mb-5 border-b border-[var(--color-paper-200)]">
        {TABS.map((t) => {
          const isActive = tab === t.id;
          return (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              className={cn(
                "inline-flex items-center gap-2 px-4 py-2 text-[13px] font-medium transition-paper -mb-px border-b-2",
                isActive
                  ? "border-[var(--color-ink-800)] text-[var(--color-ink-800)]"
                  : "border-transparent text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)]",
              )}
            >
              {t.label}
              <span
                className={cn(
                  "inline-flex items-center justify-center min-w-[22px] h-[18px] px-1.5 rounded-full",
                  "text-[11px] font-mono font-semibold tabular-nums",
                  isActive
                    ? "bg-[var(--color-ink-800)] text-white"
                    : "bg-[var(--color-paper-100)] text-[var(--color-paper-600)] border border-[var(--color-paper-300)]",
                )}
              >
                {counts[t.id]}
              </span>
            </button>
          );
        })}
      </div>

      <div className="flex items-center justify-between gap-4 mb-4">
        {tab === "kept" ? (
          <p className="text-[12px] text-[var(--color-paper-500)] leading-relaxed max-w-xl">
            Articles that passed title &amp; abstract — the corpus assessed for eligibility. Read the full
            text and <strong className="font-medium text-[var(--color-ink-700)]">Keep</strong> the eligible ones
            (confirms them for coding) or <strong className="font-medium text-[var(--color-ink-700)]">Exclude</strong> the rest.
          </p>
        ) : <span />}
        {filtered.length > 0 && (
          <div className="flex items-center gap-1 shrink-0">
            <Button variant="ghost" size="sm" onClick={() => setPage((p) => Math.max(0, p - 1))} disabled={currentPage === 0}>
              <ArrowLeft size={13} strokeWidth={2} />
              Prev
            </Button>
            <span className="px-3 text-[12px] font-semibold tabular-nums text-[var(--color-ink-700)]">
              {currentPage + 1} / {totalPages}
            </span>
            <Button variant="ghost" size="sm" onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))} disabled={currentPage >= totalPages - 1}>
              Next
              <ArrowRight size={13} strokeWidth={2} />
            </Button>
          </div>
        )}
      </div>

      {loading ? (
        <div className="flex items-center gap-2 text-[13px] text-[var(--color-paper-500)]"><Loader2 size={14} className="animate-spin" /> Loading…</div>
      ) : filtered.length === 0 ? (
        <p className="text-[13px] text-[var(--color-paper-500)] italic">
          {tab === "kept"
            ? "No included articles yet — include some at title & abstract first."
            : tab === "pending"
            ? "Nothing pending title & abstract."
            : "No excluded articles."}
        </p>
      ) : (
        <div className="space-y-3">{pageItems.map(renderItem)}</div>
      )}
    </div>
  );
}
