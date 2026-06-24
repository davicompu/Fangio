import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { listen } from "@tauri-apps/api/event";
import { confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
import {
  BookText,
  Check,
  ChevronDown,
  ChevronUp,
  Loader2,
  Pause,
  Play,
  Sparkles,
  X,
} from "lucide-react";
import { api, type AiCodingCoverage, type DimensionInfo, type FrameworkCodeDefinition, type LmModel } from "../lib/api";
import type { AiCodingHook } from "../hooks/useAiCoding";
import { cn } from "../lib/utils";
import { Button } from "./ui/Button";

interface Props {
  open: boolean;
  onClose: () => void;
  coding: AiCodingHook;
  availableModels: LmModel[];
  availableDimensions: DimensionInfo[];
  defaultChatModel: string;
  onSendToBackground: () => void;
}

const PHASE_LABELS: Record<string, string> = {
  "indexing": "Indexing PDF",
  "coding-rater1": "Rater 1 coding",
  "coding-rater2": "Rater 2 coding",
  "resolving": "Resolving",
  "adjudicating": "Adjudicating disagreements",
  "consolidating": "Consolidating codes",
};

export function AiCodingModal({
  open,
  onClose,
  coding,
  availableModels,
  availableDimensions,
  defaultChatModel,
  onSendToBackground,
}: Props) {
  const [rater1Model, setRater1Model] = useState(defaultChatModel);
  const [rater2Model, setRater2Model] = useState(defaultChatModel);
  const [dualRater, setDualRater] = useState(true);
  const [selectedDims, setSelectedDims] = useState<Set<string>>(
    new Set(availableDimensions.map((d) => d.name)),
  );
  const [coverage, setCoverage] = useState<AiCodingCoverage | null>(null);
  const [loadingCoverage, setLoadingCoverage] = useState(false);
  const [undoneConsolidations, setUndoneConsolidations] = useState<Set<number>>(new Set());

  // Codebook construction (Step 1) — runs before coding to agree a shared vocabulary.
  const [building, setBuilding] = useState(false);
  const [cbProgress, setCbProgress] = useState<{ done: number; total: number; phase: string; detail: string } | null>(null);
  const [cbMsg, setCbMsg] = useState<string | null>(null);
  const [codebookReady, setCodebookReady] = useState(false);
  const [codebookDefs, setCodebookDefs] = useState<FrameworkCodeDefinition[]>([]);
  const [newCode, setNewCode] = useState<Record<string, string>>({});
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editingValue, setEditingValue] = useState("");
  const [showRebuild, setShowRebuild] = useState(false);
  useEffect(() => {
    const un = listen<{ done: number; total: number; phase: string; detail: string }>("codebook:progress", (e) => setCbProgress(e.payload));
    return () => { un.then((f) => f()); };
  }, []);
  const [cbCoverage, setCbCoverage] = useState<{ total_with_pdf: number; participated: number; missing: number; has_codebook: boolean; any_coded: boolean; rater1_model: string | null; rater2_model: string | null } | null>(null);
  const loadCodebook = async () => {
    try {
      // Unify code casing so the codebook reads consistently regardless of which
      // rater proposed each label (and fixes codebooks built before this).
      await api.normalizeCodebookCasing().catch(() => {});
      const [defs, cov] = await Promise.all([api.listFrameworkCodeDefs(), api.codebookCoverage()]);
      setCodebookDefs(defs);
      setCbCoverage(cov);
      const ready = defs.some((d) => selectedDims.has(d.dimension));
      setCodebookReady(ready);
      // Coding is locked to the models that built the codebook — adopt them so
      // Step 3 codes with the same raters (even across sessions).
      if (ready && cov.rater1_model) {
        setRater1Model(cov.rater1_model);
        setDualRater(cov.rater2_model != null);
        if (cov.rater2_model) setRater2Model(cov.rater2_model);
      }
    } catch { /* ignore */ }
  };
  // Detect whether a codebook already exists for the selected dimensions (Step 1 done).
  useEffect(() => { if (open) void loadCodebook(); }, [open]);

  const addCodeToBook = async (dim: string) => {
    const code = (newCode[dim] ?? "").trim();
    if (!code) return;
    try { await api.addFrameworkCodeDef(dim, code); setNewCode((m) => ({ ...m, [dim]: "" })); await loadCodebook(); } catch { /* ignore */ }
  };
  const removeCodeFromBook = async (id: number) => {
    try { await api.deleteFrameworkCodeDef(id); await loadCodebook(); } catch { /* ignore */ }
  };
  const commitRename = async () => {
    if (editingId == null) return;
    const next = editingValue.trim();
    const current = codebookDefs.find((d) => d.id === editingId);
    setEditingId(null);
    if (!next || !current || next === current.code) return;
    try { await api.renameFrameworkCodeDef(editingId, next); await loadCodebook(); } catch { /* ignore */ }
  };

  const handleBuildCodebook = async (onlyMissing = false) => {
    if (selectedDims.size === 0) return;
    // Rebuilding from scratch replaces the shared vocabulary, so any coding done
    // against the old codebook is stale. Confirm and reset it before continuing.
    const isRebuild = !onlyMissing && codebookReady;
    let resetCoding = false;
    if (isRebuild && cbCoverage?.any_coded) {
      const ok = await confirmDialog(
        "Rebuilding replaces the shared codebook. Existing AI coding was done against the old codebook, so it will be reset — you'll re-run coding afterward. Your own (researcher) codes are kept. Continue?",
        { title: "Reset coding and rebuild?", kind: "warning", okLabel: "Reset & rebuild", cancelLabel: "Cancel" },
      );
      if (!ok) return;
      resetCoding = true;
    }
    setBuilding(true);
    setCbMsg(null);
    setCbProgress({ done: 0, total: 0, phase: "proposing", detail: "" });
    try {
      if (resetCoding) await api.resetAiCoding();
      const r = await api.buildCodebook(rater1Model, dualRater ? rater2Model : null, [...selectedDims], { onlyMissing });
      await loadCodebook();
      setCodebookReady(true);
      setShowRebuild(false);
      const base = onlyMissing
        ? `Extended the codebook from ${r.sampled} new article${r.sampled === 1 ? "" : "s"} — ${r.codes_added} code${r.codes_added === 1 ? "" : "s"} added.`
        : `Codebook built from ${r.sampled} article${r.sampled === 1 ? "" : "s"} — ${r.codes_added} new code${r.codes_added === 1 ? "" : "s"} added.`;
      const recode = r.codes_added > 0 && cbCoverage?.any_coded ? " The codebook changed — re-run coding so every article is coded against it." : " Review and edit the codes below, then start coding.";
      setCbMsg(base + recode);
    } catch (e: any) {
      const m: string = e?.message ?? String(e);
      setCbMsg(m.includes("reasoning model") ? "Pick a reasoning model in Settings first." : `Couldn't build codebook: ${m}`);
    } finally {
      setBuilding(false);
      setCbProgress(null);
    }
  };

  // Load coverage when the modal opens in idle state
  useEffect(() => {
    if (!open || coding.step !== "idle") return;
    setLoadingCoverage(true);
    api.getAiCodingCoverage()
      .then(setCoverage)
      .catch(() => setCoverage(null))
      .finally(() => setLoadingCoverage(false));
  }, [open, coding.step]);

  // Keep selectedDims in sync if dimensions change
  useEffect(() => {
    setSelectedDims(new Set(availableDimensions.map((d) => d.name)));
  }, [availableDimensions]);

  // Fresh run → clear any undo state from a previous run's summary
  useEffect(() => {
    if (coding.step === "running") setUndoneConsolidations(new Set());
  }, [coding.step]);

  const toggleDim = (name: string) => {
    setSelectedDims((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name); else next.add(name);
      return next;
    });
  };

  // Rater model pickers — shown when first building the codebook and again under
  // "Rebuild", since coding is locked to whichever models built the codebook.
  const ratersEditor = (
    <div>
      <p className="text-[11px] font-semibold uppercase tracking-wider text-[var(--color-paper-500)] mb-1.5">Raters</p>
      <div className="space-y-2">
        <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-200)] px-3 py-2.5">
          <p className="text-[11px] font-semibold text-[var(--color-paper-500)] mb-1.5">Rater 1</p>
          <select value={rater1Model} onChange={(e) => setRater1Model(e.target.value)} className="w-full text-[12px] px-2.5 py-1.5 rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white text-[var(--color-ink-800)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-400)]">
            {availableModels.length === 0 && <option value={rater1Model}>{rater1Model}</option>}
            {availableModels.map((m) => <option key={m.id} value={m.id}>{m.id}</option>)}
          </select>
        </div>
        <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-200)] overflow-hidden">
          <button onClick={() => setDualRater((v) => !v)} className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-[var(--color-paper-50)] transition-paper">
            <div>
              <p className="text-[11px] font-semibold text-[var(--color-paper-500)]">Rater 2 {dualRater ? "" : "(disabled)"}</p>
              {!dualRater && <p className="text-[10px] text-[var(--color-paper-400)] mt-0.5">With one rater there's no agreement to measure — its codes apply directly.</p>}
            </div>
            {dualRater ? <ChevronUp size={13} className="text-[var(--color-paper-400)]" /> : <ChevronDown size={13} className="text-[var(--color-paper-400)]" />}
          </button>
          <AnimatePresence>
            {dualRater && (
              <motion.div initial={{ height: 0, opacity: 0 }} animate={{ height: "auto", opacity: 1 }} exit={{ height: 0, opacity: 0 }} transition={{ duration: 0.15 }} className="overflow-hidden border-t border-[var(--color-paper-200)] px-3 py-2.5">
                <select value={rater2Model} onChange={(e) => setRater2Model(e.target.value)} className="w-full text-[12px] px-2.5 py-1.5 rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white text-[var(--color-ink-800)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-400)]">
                  {availableModels.length === 0 && <option value={rater2Model}>{rater2Model}</option>}
                  {availableModels.map((m) => <option key={m.id} value={m.id}>{m.id}</option>)}
                </select>
                <p className="mt-1.5 text-[10px] text-[var(--color-paper-500)] leading-relaxed">Both raters code against the codebook. Agreement (Cohen's κ) is reported, and articles where the raters conflict are highlighted for your review afterward.</p>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>
    </div>
  );

  const handleStart = () => {
    if (selectedDims.size === 0) return;
    // Coding is locked to the models that built the codebook (falls back to the
    // current selection only if, somehow, the build models weren't recorded).
    const r1 = cbCoverage?.rater1_model ?? rater1Model;
    const r2 = cbCoverage?.rater1_model
      ? cbCoverage.rater2_model
      : (dualRater ? rater2Model : null);
    coding.start({
      rater1Model: r1,
      rater2Model: r2,
      dimensions: [...selectedDims],
    });
  };

  // Closing mid-run STOPS the work (distinct from "Send to background", which
  // keeps it going). Warn first, and only stop + close on confirmation.
  const requestClose = async () => {
    if (coding.step === "running") {
      const ok = await confirmDialog(
        "Closing will stop the coding run — you'll have to start it again from the beginning. To keep it going instead, use “Send to background”.",
        { title: "Stop coding and close?", kind: "warning", okLabel: "Stop & close", cancelLabel: "Keep running" },
      );
      if (!ok) return;
      await coding.cancel();
    } else if (building) {
      const ok = await confirmDialog(
        "A codebook build is still running and can't be cancelled — closing will leave it to finish on its own. Close anyway?",
        { title: "Build in progress", kind: "warning", okLabel: "Close", cancelLabel: "Wait" },
      );
      if (!ok) return;
    }
    onClose();
  };

  if (!open) return null;

  const progress = coding.progress;
  const pct = progress && progress.total > 0
    ? Math.round((progress.done / progress.total) * 100)
    : 0;

  // Group consolidations by canonical label (per dimension) so each canonical
  // shows once with its merged variants nested beneath — far easier to scan.
  const consolidationGroups = (() => {
    const live = (coding.result?.consolidations ?? []).filter((c) => !undoneConsolidations.has(c.id));
    const map = new Map<string, { dimension: string; canonical: string; items: { id: number; variant: string }[] }>();
    for (const c of live) {
      const key = `${c.dimension}␟${c.canonical_code}`;
      if (!map.has(key)) map.set(key, { dimension: c.dimension, canonical: c.canonical_code, items: [] });
      map.get(key)!.items.push({ id: c.id, variant: c.variant_code });
    }
    return [...map.values()];
  })();

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/40 backdrop-blur-sm">
      <motion.div
        initial={{ opacity: 0, scale: 0.97, y: 8 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.97, y: 8 }}
        transition={{ duration: 0.18 }}
        className="w-full max-w-2xl bg-white rounded-[var(--radius-lg)] border border-[var(--color-paper-300)] shadow-[var(--shadow-paper-lg)] overflow-hidden flex flex-col max-h-[88vh]"
      >
        {/* Header */}
        <div className="flex items-center gap-2.5 px-5 py-4 border-b border-[var(--color-paper-200)] shrink-0">
          <Sparkles size={16} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
          <h2 className="font-semibold text-[15px] text-[var(--color-ink-900)] flex-1">
            Code with AI Raters
          </h2>
          <button
            onClick={requestClose}
            className="p-1 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] text-[var(--color-paper-500)] transition-paper"
          >
            <X size={14} strokeWidth={2} />
          </button>
        </div>

        {/* Body */}
        <div className="px-5 py-4 space-y-5 flex-1 overflow-y-auto min-h-0">

          {/* Codebook build progress — prominent, at the top while it runs. */}
          {coding.step === "idle" && building && (
            <div className="rounded-[var(--radius-md)] border border-[var(--color-accent-200)] bg-[var(--color-accent-50)] px-4 py-3">
              <div className="flex items-center justify-between mb-1.5">
                <span className="text-[12px] font-semibold text-[var(--color-accent-700)] inline-flex items-center gap-2">
                  <Loader2 size={13} className="animate-spin" />
                  {cbProgress?.phase === "consolidating" ? "Consolidating codes…" : "Raters proposing codes…"}
                </span>
                {cbProgress && cbProgress.total > 0 && (
                  <span className="text-[11px] tabular-nums text-[var(--color-accent-600)]">{cbProgress.done} / {cbProgress.total}</span>
                )}
              </div>
              <div className="h-1.5 w-full rounded-full bg-[var(--color-accent-100)] overflow-hidden">
                <div
                  className="h-full bg-[var(--color-accent-500)] transition-[width] duration-300"
                  style={{ width: cbProgress && cbProgress.total > 0 ? `${Math.round((cbProgress.done / cbProgress.total) * 100)}%` : "8%" }}
                />
              </div>
              {cbProgress?.detail && <p className="mt-1.5 text-[11px] text-[var(--color-accent-700)]/70 truncate">{cbProgress.detail}</p>}
            </div>
          )}

          {/* Codebook result message (after a build completes). */}
          {coding.step === "idle" && !building && cbMsg && (
            <div className="rounded-[var(--radius-sm)] border border-[var(--color-ok-200)] bg-[var(--color-ok-50)] px-3.5 py-2.5 text-[12px] text-[var(--color-ok-700)]">
              {cbMsg}
            </div>
          )}

          {/* ── Config form (idle) ── */}
          {coding.step === "idle" && (() => {
            const marker = (state: "done" | "active" | "upcoming", n: number) => (
              <span className={cn(
                "inline-flex items-center justify-center w-6 h-6 rounded-full text-[11px] font-semibold shrink-0",
                state === "done" ? "bg-[var(--color-ok-500)] text-white" : state === "active" ? "bg-[var(--color-ink-800)] text-white" : "bg-[var(--color-paper-200)] text-[var(--color-paper-500)]",
              )}>
                {state === "done" ? <Check size={12} strokeWidth={3} /> : n}
              </span>
            );
            const line = <div className="w-px flex-1 bg-[var(--color-paper-300)] my-1.5" />;
            const title = (text: string, on: boolean) => (
              <div className={cn("text-[13px] font-semibold leading-6", on ? "text-[var(--color-ink-900)]" : "text-[var(--color-paper-500)]")}>{text}</div>
            );
            return (
              <div>
                {/* ── Step 1 · Build codebook ───────────────────────────── */}
                <div className="flex gap-3">
                  <div className="flex flex-col items-center">
                    {marker(codebookReady ? "done" : "active", 1)}
                    {line}
                  </div>
                  <div className="flex-1 min-w-0 pb-5">
                    {title("Build codebook", true)}
                    {!codebookReady ? (
                      <div className="mt-2 space-y-4">
                        <p className="text-[12px] text-[var(--color-paper-500)] leading-relaxed">
                          The raters agree a shared codebook over the included articles that have a PDF, so both code against the same vocabulary.
                        </p>
                        {/* Coverage */}
                        <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-200)] bg-[var(--color-paper-50)] px-3.5 py-2.5 space-y-1.5">
                          {loadingCoverage && (
                            <div className="flex items-center gap-2 text-[12px] text-[var(--color-paper-500)]"><Loader2 size={12} className="animate-spin" /> Loading…</div>
                          )}
                          {coverage && !loadingCoverage && (
                            <>
                              <div className="flex items-center gap-2 text-[12px] text-[var(--color-ok-700)]">
                                <Check size={12} strokeWidth={2.5} />
                                <span>{coverage.ready} article{coverage.ready !== 1 ? "s" : ""} ready to code</span>
                              </div>
                              {coverage.needs_indexing > 0 && (
                                <p className="text-[12px] text-[var(--color-warn-700)]">↻ {coverage.needs_indexing} need indexing — done automatically</p>
                              )}
                              {coverage.no_pdf > 0 && (
                                <p className="text-[12px] text-[var(--color-paper-500)]">⊘ {coverage.no_pdf} missing PDF — will be skipped</p>
                              )}
                            </>
                          )}
                        </div>
                        {/* Dimensions */}
                        <div>
                          <p className="text-[11px] font-semibold uppercase tracking-wider text-[var(--color-paper-500)] mb-1.5">Dimensions</p>
                          <div className="flex flex-wrap gap-1.5">
                            {availableDimensions.map((d) => (
                              <button key={d.name} onClick={() => toggleDim(d.name)} className={cn(
                                "px-2.5 py-1 rounded-full text-[11px] font-medium border transition-paper",
                                selectedDims.has(d.name) ? "bg-[var(--color-ink-800)] border-[var(--color-ink-800)] text-white" : "bg-white border-[var(--color-paper-300)] text-[var(--color-paper-600)] hover:border-[var(--color-ink-400)]",
                              )}>{d.name}</button>
                            ))}
                          </div>
                        </div>
                        {/* Raters */}
                        {ratersEditor}
                      </div>
                    ) : (
                      <div className="mt-1">
                        <p className="text-[12px] text-[var(--color-paper-500)]">
                          {cbCoverage ? `Built from ${cbCoverage.participated} of ${cbCoverage.total_with_pdf} articles with a PDF.` : "Codebook ready."}
                          <button onClick={() => setShowRebuild((v) => !v)} className="ml-2 text-[var(--color-accent-600)] hover:text-[var(--color-accent-700)] font-medium">{showRebuild ? "Close" : "Rebuild"}</button>
                        </p>
                        <AnimatePresence>
                          {showRebuild && (
                            <motion.div initial={{ height: 0, opacity: 0 }} animate={{ height: "auto", opacity: 1 }} exit={{ height: 0, opacity: 0 }} transition={{ duration: 0.18 }} className="overflow-hidden">
                              <div className="mt-3 space-y-3">
                                <p className="text-[11px] text-[var(--color-paper-500)] leading-relaxed">Pick the raters, then rebuild. Coding will be locked to these models.</p>
                                {ratersEditor}
                                <Button variant="primary" size="sm" onClick={() => handleBuildCodebook(false)} disabled={building}>
                                  {building ? <Loader2 size={13} className="animate-spin" /> : <BookText size={13} strokeWidth={1.75} />}
                                  {building ? "Rebuilding…" : "Rebuild codebook"}
                                </Button>
                              </div>
                            </motion.div>
                          )}
                        </AnimatePresence>
                      </div>
                    )}
                  </div>
                </div>

                {/* ── Step 2 · Review proposed codes ────────────────────── */}
                <div className="flex gap-3">
                  <div className="flex flex-col items-center">
                    {marker(codebookReady ? "active" : "upcoming", 2)}
                    {line}
                  </div>
                  <div className="flex-1 min-w-0 pb-5">
                    {title("Review proposed codes", codebookReady)}
                    {codebookReady ? (
                      <div className="mt-2">
                        {cbCoverage && cbCoverage.missing > 0 && (
                          <button onClick={() => handleBuildCodebook(true)} disabled={building} className="mb-2 inline-flex items-center gap-1 text-[11px] font-medium text-[var(--color-accent-600)] hover:text-[var(--color-accent-700)] disabled:opacity-40">
                            {building ? <Loader2 size={11} className="animate-spin" /> : <BookText size={11} strokeWidth={1.75} />}
                            Add codes from {cbCoverage.missing} new article{cbCoverage.missing === 1 ? "" : "s"}
                          </button>
                        )}
                        <div className="space-y-4">
                          {[...selectedDims].map((dim) => {
                            const codes = codebookDefs.filter((d) => d.dimension === dim);
                            return (
                              <div key={dim}>
                                <div className="text-[12px] font-medium text-[var(--color-ink-800)] mb-1.5">{dim}</div>
                                <div className="flex flex-wrap gap-1.5">
                                  {codes.length === 0 && <span className="text-[11px] text-[var(--color-paper-400)] italic">No codes yet — add some below.</span>}
                                  {codes.map((c) => (
                                    <span key={c.id} className="inline-flex items-center gap-1 pl-2.5 pr-1.5 py-1 rounded-full text-[11px] bg-[var(--color-paper-100)] border border-[var(--color-paper-300)] text-[var(--color-ink-700)]">
                                      {editingId === c.id ? (
                                        <input
                                          autoFocus
                                          type="text"
                                          value={editingValue}
                                          onChange={(e) => setEditingValue(e.target.value)}
                                          onBlur={commitRename}
                                          onKeyDown={(e) => { if (e.key === "Enter") commitRename(); else if (e.key === "Escape") setEditingId(null); }}
                                          className="bg-transparent border-0 p-0 m-0 text-[11px] text-[var(--color-ink-800)] focus:outline-none"
                                          style={{ width: `${Math.max(editingValue.length, 4) + 1}ch` }}
                                        />
                                      ) : (
                                        <button onClick={() => { setEditingId(c.id); setEditingValue(c.code); }} className="hover:text-[var(--color-ink-900)] hover:underline decoration-dotted underline-offset-2" title="Click to rephrase">{c.code}</button>
                                      )}
                                      <button onClick={() => removeCodeFromBook(c.id)} className="rounded-full p-0.5 hover:bg-[var(--color-paper-300)] text-[var(--color-paper-500)] hover:text-[var(--color-error-500)]" title="Remove from codebook"><X size={10} strokeWidth={2.5} /></button>
                                    </span>
                                  ))}
                                </div>
                                <div className="mt-1.5 flex items-center gap-1.5">
                                  <input type="text" value={newCode[dim] ?? ""} onChange={(e) => setNewCode((m) => ({ ...m, [dim]: e.target.value }))} onKeyDown={(e) => { if (e.key === "Enter") addCodeToBook(dim); }} placeholder="Add a code…" className="flex-1 px-2.5 py-1 text-[11px] rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white focus:outline-none focus:ring-1 focus:ring-[var(--color-paper-400)]" />
                                  <button onClick={() => addCodeToBook(dim)} className="px-2 py-1 text-[11px] font-medium text-[var(--color-accent-600)] hover:text-[var(--color-accent-700)]">Add</button>
                                </div>
                              </div>
                            );
                          })}
                        </div>
                        <p className="mt-2 text-[10px] text-[var(--color-paper-400)] leading-relaxed">Click a code to rephrase it; the × removes it. Edits apply to the Organisation codebook, and coding will use exactly these codes.</p>
                      </div>
                    ) : (
                      <p className="mt-1 text-[12px] text-[var(--color-paper-500)]">Edit and confirm the proposed codebook before coding.</p>
                    )}
                  </div>
                </div>

                {/* ── Step 3 · Code corpus ──────────────────────────────── */}
                <div className="flex gap-3">
                  <div className="flex flex-col items-center">{marker(codebookReady ? "active" : "upcoming", 3)}</div>
                  <div className="flex-1 min-w-0">
                    {title("Code corpus", codebookReady)}
                    <p className="mt-1 text-[12px] text-[var(--color-paper-500)] leading-relaxed">
                      {dualRater
                        ? "Both raters code every article against the codebook. Agreement (Cohen's κ) is reported, and conflicting articles are highlighted for your review."
                        : "The rater codes every article against the codebook; its codes apply directly."}
                    </p>
                    {codebookReady && (
                      <div className="mt-2 rounded-[var(--radius-sm)] border border-[var(--color-paper-200)] bg-[var(--color-paper-50)] px-3 py-2 space-y-1">
                        <p className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-paper-400)]">Coding with the raters that built the codebook</p>
                        <div className="flex items-center gap-2 text-[12px] text-[var(--color-ink-800)]">
                          <span className="text-[var(--color-paper-500)]">Rater 1</span>
                          <span className="font-medium tabular-nums">{rater1Model}</span>
                        </div>
                        {dualRater && (
                          <div className="flex items-center gap-2 text-[12px] text-[var(--color-ink-800)]">
                            <span className="text-[var(--color-paper-500)]">Rater 2</span>
                            <span className="font-medium tabular-nums">{rater2Model}</span>
                          </div>
                        )}
                        <p className="text-[10px] text-[var(--color-paper-400)] leading-relaxed pt-0.5">To code with different models, Rebuild the codebook above with your chosen raters.</p>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            );
          })()}

          {/* ── Running ── */}
          {coding.step === "running" && (
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                {coding.isPaused
                  ? <Pause size={14} strokeWidth={2} className="text-[var(--color-warn-500)]" />
                  : <Loader2 size={14} className="animate-spin text-[var(--color-accent-500)]" />
                }
                <span className="text-[13px] font-medium text-[var(--color-ink-800)]">
                  {coding.isPaused ? "Paused" : (progress ? (PHASE_LABELS[progress.phase] ?? "Coding") : "Starting…")}
                </span>
              </div>

              {progress && (
                <>
                  <p className="text-[12px] text-[var(--color-paper-600)] truncate">
                    {progress.article_title}
                  </p>
                  <div className="space-y-1">
                    <div className="flex justify-between text-[11px] text-[var(--color-paper-500)]">
                      <span>{progress.done} / {progress.total} articles</span>
                      <span>{pct}%</span>
                    </div>
                    <div className="h-1.5 rounded-full bg-[var(--color-paper-200)] overflow-hidden">
                      <motion.div
                        animate={{ width: `${pct}%` }}
                        transition={{ duration: 0.3 }}
                        className="h-full rounded-full bg-[var(--color-accent-500)]"
                      />
                    </div>
                  </div>
                  <div className="flex gap-4 text-[11px] text-[var(--color-paper-500)]">
                    {progress.consensus > 0 && (
                      <span className="text-[var(--color-ok-600)]">✓ {progress.consensus} consensus</span>
                    )}
                    {progress.disagreements > 0 && (
                      <span className="text-amber-600">⚠ {progress.disagreements} need review</span>
                    )}
                    {progress.skipped_no_pdf > 0 && (
                      <span>⊘ {progress.skipped_no_pdf} skipped</span>
                    )}
                  </div>
                </>
              )}
            </div>
          )}

          {/* ── Done ── */}
          {coding.step === "done" && coding.result && (
            <div className="space-y-2 text-[13px]">
              <div className="flex items-center gap-2 text-[var(--color-ok-700)]">
                <Check size={16} strokeWidth={2.5} />
                <span className="font-semibold">Coding complete</span>
              </div>
              <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-200)] divide-y divide-[var(--color-paper-200)]">
                <div className="flex justify-between px-3 py-2 text-[12px]">
                  <span className="text-[var(--color-paper-600)]">Articles processed</span>
                  <span className="font-medium">{coding.result.processed}</span>
                </div>
                <div className="flex justify-between px-3 py-2 text-[12px]">
                  <span className="text-[var(--color-ok-600)]">Consensus codes applied</span>
                  <span className="font-medium text-[var(--color-ok-700)]">{coding.result.consensus}</span>
                </div>
                {coding.result.adjudicated > 0 && (
                  <div className="flex justify-between px-3 py-2 text-[12px]">
                    <span className="text-[var(--color-ok-600)]">Resolved by adjudicator</span>
                    <span className="font-medium text-[var(--color-ok-700)]">{coding.result.adjudicated}</span>
                  </div>
                )}
                {coding.result.consolidated > 0 && (
                  <div className="flex justify-between px-3 py-2 text-[12px]">
                    <span className="text-[var(--color-ok-600)]">Codes consolidated</span>
                    <span className="font-medium text-[var(--color-ok-700)]">{coding.result.consolidated}</span>
                  </div>
                )}
                {coding.result.disagreements > 0 && (
                  <div className="flex justify-between px-3 py-2 text-[12px]">
                    <span className="text-amber-600">Disagreements to review</span>
                    <span className="font-medium text-amber-700">{coding.result.disagreements}</span>
                  </div>
                )}
                {coding.result.skipped_no_pdf > 0 && (
                  <div className="flex justify-between px-3 py-2 text-[12px]">
                    <span className="text-[var(--color-paper-500)]">Skipped (no PDF)</span>
                    <span className="font-medium">{coding.result.skipped_no_pdf}</span>
                  </div>
                )}
                {coding.result.indexed > 0 && (
                  <div className="flex justify-between px-3 py-2 text-[12px]">
                    <span className="text-[var(--color-paper-500)]">PDFs indexed during run</span>
                    <span className="font-medium">{coding.result.indexed}</span>
                  </div>
                )}
              </div>

              {/* Consolidated codes — grouped by canonical, variants nested with subtle undo */}
              {consolidationGroups.length > 0 && (
                <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-200)] bg-[var(--color-paper-50)] px-3 py-2.5 max-h-56 overflow-y-auto">
                  <p className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-paper-500)] mb-2">
                    Consolidated codes
                  </p>
                  <div className="space-y-2.5">
                    {consolidationGroups.map((g) => (
                      <div key={`${g.dimension}-${g.canonical}`}>
                        <div className="flex items-baseline gap-1.5">
                          <span className="text-[12px] font-semibold text-[var(--color-ink-900)]">{g.canonical}</span>
                          <span className="text-[10px] uppercase tracking-wider text-[var(--color-paper-400)]">{g.dimension}</span>
                        </div>
                        <ul className="mt-0.5 ml-2 border-l border-[var(--color-paper-300)]">
                          {g.items.map((it) => (
                            <li key={it.id} className="flex items-center gap-1.5 pl-2.5 py-0.5 text-[11px] leading-snug">
                              <span className="text-[var(--color-paper-400)]">was</span>
                              <span className="text-[var(--color-paper-600)] truncate">{it.variant}</span>
                              <button
                                onClick={async () => {
                                  await api.undoConsolidation(it.id);
                                  setUndoneConsolidations((prev) => new Set(prev).add(it.id));
                                }}
                                className="ml-auto shrink-0 text-[10px] text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)] underline underline-offset-2 transition-paper"
                              >
                                undo
                              </button>
                            </li>
                          ))}
                        </ul>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* ── Error ── */}
          {coding.step === "error" && (
            <p className="text-[12px] text-[var(--color-error-600)] leading-relaxed">
              {coding.error ?? "An unexpected error occurred."}
            </p>
          )}
        </div>


        {/* Footer */}
        <div className="flex items-center justify-between gap-2 px-5 py-3 border-t border-[var(--color-paper-200)] bg-[var(--color-paper-50)] shrink-0">
          {coding.step === "idle" && (
            <>
              <Button variant="ghost" size="sm" onClick={onClose}>Cancel</Button>
              {!codebookReady ? (
                // Step 1 — build the shared codebook.
                <Button
                  variant="primary"
                  size="sm"
                  onClick={() => handleBuildCodebook(false)}
                  disabled={building || selectedDims.size === 0}
                  title="Sample the corpus and build a shared codebook. You'll review the codes before coding."
                >
                  {building ? <Loader2 size={13} className="animate-spin" /> : <BookText size={13} strokeWidth={1.75} />}
                  {building ? "Building…" : "Step 1 · Build codebook"}
                </Button>
              ) : (
                // Step 3 — code against the reviewed codebook (rebuild lives in Step 1).
                <Button
                  variant="primary"
                  size="sm"
                  onClick={handleStart}
                  disabled={building || selectedDims.size === 0 || loadingCoverage || (coverage !== null && coverage.ready === 0 && coverage.needs_indexing === 0)}
                >
                  <Sparkles size={13} strokeWidth={1.75} />
                  Step 3 · Start coding
                </Button>
              )}
            </>
          )}

          {coding.step === "running" && (
            <>
              <button
                onClick={() => coding.isPaused ? coding.resume() : coding.pause()}
                className="flex items-center gap-1.5 text-[12px] text-[var(--color-paper-600)] hover:text-[var(--color-ink-800)] transition-paper"
              >
                {coding.isPaused
                  ? <><Play size={12} strokeWidth={2} /> Resume</>
                  : <><Pause size={12} strokeWidth={2} /> Pause</>
                }
              </button>
              <div className="flex items-center gap-2">
                <button
                  onClick={onSendToBackground}
                  className="text-[12px] text-[var(--color-accent-600)] hover:text-[var(--color-accent-700)] font-medium transition-paper"
                >
                  Send to background
                </button>
                <button
                  onClick={() => coding.cancel()}
                  className="text-[12px] text-[var(--color-error-500)] hover:text-[var(--color-error-700)] transition-paper"
                >
                  Cancel
                </button>
              </div>
            </>
          )}

          {(coding.step === "done" || coding.step === "error") && (
            <>
              <span />
              <Button variant="primary" size="sm" onClick={onClose}>
                Done
              </Button>
            </>
          )}
        </div>
      </motion.div>
    </div>
  );
}
