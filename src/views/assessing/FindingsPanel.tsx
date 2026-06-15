import { useEffect, useMemo, useState } from "react";
import { BarChart3, Check, Loader2 } from "lucide-react";
import { api, type CodingRow, type DimensionInfo, type ProjectRow } from "../../lib/api";
import { Select } from "../../components/ui/Select";

const ANALYSIS_METHODS = [
  "Content analysis",
  "Thematic analysis",
  "Framework-based analysis (TCM)",
  "Framework-based analysis (ADO)",
  "Bibliometric analysis",
  "Meta-analysis",
  "Narrative synthesis",
];

/** Evaluation (SPAR-4-SLR 3a): the evidence base — code frequencies per
 * framework dimension from Stage 2 — plus the recorded analysis method. */
export function FindingsPanel({ project, onSaved }: { project: ProjectRow; onSaved?: () => void | Promise<void> }) {
  const [dimensions, setDimensions] = useState<DimensionInfo[]>([]);
  const [codings, setCodings] = useState<CodingRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [method, setMethod] = useState(project.analysis_method ?? "");
  const [savedMethod, setSavedMethod] = useState(false);
  const [showPicker, setShowPicker] = useState(false);

  // The framework you chose already implies a framework-based analysis — no need
  // to ask again. Derive the default; the picker is only for the rare override.
  const frameworkMethod =
    project.framework === "TCM" ? "Framework-based analysis (TCM)"
    : project.framework === "ADO" ? "Framework-based analysis (ADO)"
    : project.framework === "Custom" ? "Framework-based analysis (custom framework)"
    : project.framework ? `Framework-based analysis (${project.framework})`
    : "Framework-based analysis";
  const overridden = !!method.trim();
  const effectiveMethod = overridden ? method : frameworkMethod;

  useEffect(() => {
    Promise.all([api.listFrameworkDimensions(), api.listCodingBatch(0, 9999)])
      .then(([dims, batch]) => {
        setDimensions(dims);
        setCodings(batch.codings);
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const byDimension = useMemo(() => {
    const map = new Map<string, Map<string, number>>();
    for (const c of codings) {
      if (!map.has(c.dimension)) map.set(c.dimension, new Map());
      const m = map.get(c.dimension)!;
      m.set(c.code, (m.get(c.code) ?? 0) + 1);
    }
    return map;
  }, [codings]);

  const totalCodes = codings.length;
  const codedArticles = useMemo(() => new Set(codings.map((c) => c.article_id)).size, [codings]);

  const saveMethod = async (value: string) => {
    setMethod(value);
    try {
      await api.saveDecision({
        stage: "assessing",
        sub_stage: "evaluation",
        decision_key: "project.analysis_method",
        value_json: JSON.stringify(value),
        rationale: null,
      });
      await onSaved?.();
      setSavedMethod(true);
      setTimeout(() => setSavedMethod(false), 1500);
    } catch {
      // ignore — non-fatal
    }
  };

  return (
    <div className="space-y-8 max-w-3xl">
      {/* Analysis method */}
      <section>
        <div className="flex items-center gap-2 mb-1.5 text-[var(--color-paper-500)]">
          <BarChart3 size={14} strokeWidth={1.75} />
          <span className="eyebrow">Analysis method</span>
          {savedMethod && (
            <span className="inline-flex items-center gap-1 text-[11px] text-[var(--color-ok-600)]">
              <Check size={11} strokeWidth={2.5} /> saved
            </span>
          )}
        </div>
        {/* Derived from the framework — not a required choice */}
        <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] px-4 py-3 max-w-xl">
          <div className="flex items-baseline gap-2">
            <span className="text-[14px] font-medium text-[var(--color-ink-900)]">{effectiveMethod}</span>
            {!overridden && (
              <span className="text-[11px] text-[var(--color-paper-500)]">
                · derived from your {project.framework ?? "chosen"} framework
              </span>
            )}
          </div>
          <p className="text-[12px] text-[var(--color-paper-500)] leading-relaxed mt-1">
            Your {project.framework ?? "chosen"} framework already defines a framework-based analysis, so there's nothing to set here — it's recorded in the manuscript's method section automatically. Override it only if you also applied a different technique (e.g. a bibliometric pass).
          </p>
          <div className="mt-2 flex items-center gap-3">
            <button
              onClick={() => setShowPicker((v) => !v)}
              className="text-[12px] text-[var(--color-accent-600)] hover:text-[var(--color-accent-700)] underline underline-offset-2"
            >
              {showPicker ? "Cancel" : overridden ? "Change method" : "Use a different method"}
            </button>
            {overridden && (
              <button
                onClick={() => { setShowPicker(false); saveMethod(""); }}
                className="text-[12px] text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] underline underline-offset-2"
              >
                Reset to framework default
              </button>
            )}
          </div>
          {showPicker && (
            <div className="mt-3 max-w-sm">
              <Select
                value={method}
                onChange={(e) => { saveMethod(e.target.value); setShowPicker(false); }}
              >
                <option value="">— framework default ({frameworkMethod}) —</option>
                {ANALYSIS_METHODS.map((m) => (
                  <option key={m} value={m}>{m}</option>
                ))}
                {method && !ANALYSIS_METHODS.includes(method) && <option value={method}>{method}</option>}
              </Select>
            </div>
          )}
        </div>
      </section>

      {/* Findings by dimension */}
      <section>
        <div className="flex items-baseline justify-between mb-3">
          <div>
            <span className="eyebrow text-[var(--color-paper-500)]">Findings by dimension</span>
            <h3 className="font-serif text-[20px] tracking-tight text-[var(--color-ink-900)] mt-0.5">
              Code frequencies
            </h3>
          </div>
          <span className="text-[12px] text-[var(--color-paper-500)] tabular-nums">
            {totalCodes} codes · {codedArticles} articles
          </span>
        </div>

        {loading ? (
          <div className="flex items-center gap-2 text-[13px] text-[var(--color-paper-500)]">
            <Loader2 size={14} className="animate-spin" /> Loading codings…
          </div>
        ) : totalCodes === 0 ? (
          <p className="text-[13px] text-[var(--color-paper-500)] italic">
            No codes applied yet. Code your included articles in Stage 2 (Arranging) first.
          </p>
        ) : (
          <div className="space-y-5">
            {dimensions.map((dim) => {
              const codes = byDimension.get(dim.name);
              if (!codes || codes.size === 0) return null;
              const rows = [...codes.entries()].sort((a, b) => b[1] - a[1]);
              const max = rows[0][1];
              return (
                <div key={dim.name} className="rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] overflow-hidden">
                  <div className="px-4 py-2.5 bg-[var(--color-paper-100)] border-b border-[var(--color-paper-200)] flex items-baseline justify-between">
                    <span className="text-[13px] font-semibold text-[var(--color-ink-800)]">{dim.name}</span>
                    <span className="text-[11px] text-[var(--color-paper-500)] tabular-nums">{rows.length} distinct</span>
                  </div>
                  <div className="px-4 py-3 space-y-1.5">
                    {rows.map(([code, n]) => (
                      <div key={code} className="flex items-center gap-3">
                        <span className="text-[12px] text-[var(--color-ink-800)] w-1/2 truncate" title={code}>{code}</span>
                        <div className="flex-1 h-2 rounded-full bg-[var(--color-paper-200)] overflow-hidden">
                          <div
                            className="h-full rounded-full bg-[var(--color-accent-400)]"
                            style={{ width: `${Math.round((n / max) * 100)}%` }}
                          />
                        </div>
                        <span className="text-[11px] text-[var(--color-paper-500)] tabular-nums w-6 text-right">{n}</span>
                      </div>
                    ))}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </section>
    </div>
  );
}
