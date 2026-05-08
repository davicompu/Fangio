import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { BookOpen, Filter, Loader2, Plus, RefreshCw, Save, Trash2, X } from "lucide-react";
import { api, type ExclusionCriterionRow, type ProjectRow } from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { TextField, TextArea } from "../../components/ui/TextField";
import { Select } from "../../components/ui/Select";

interface Props {
  project: ProjectRow;
  refreshKey: number;
  onChanged: () => void;
}

// ABS: 4* > 4 > 3 > 2 > 1  (higher index = higher quality)
const ABS_LEVELS = ["1", "2", "3", "4", "4*"] as const;
// ABDC: A* > A > B > C
const ABDC_LEVELS = ["C", "B", "A", "A*"] as const;

function parseJournalQuality(raw: string | null) {
  try {
    return raw ? (JSON.parse(raw) as { abs_min?: string; abdc_min?: string; if_min?: number }) : {};
  } catch {
    return {};
  }
}

export function CriteriaPanel({ project, onChanged }: Props) {
  const [criteria, setCriteria] = useState<ExclusionCriterionRow[]>([]);
  const [adding, setAdding] = useState<{ label: string; rationale: string } | null>(null);
  const [editing, setEditing] = useState<number | null>(null);
  const [draft, setDraft] = useState<{ label: string; rationale: string }>({
    label: "",
    rationale: "",
  });
  const [loading, setLoading] = useState(true);

  // Journal quality thresholds
  const jq = parseJournalQuality(project.journal_quality);
  const [absMin, setAbsMin] = useState<string>(jq.abs_min ?? "");
  const [abdcMin, setAbdcMin] = useState<string>(jq.abdc_min ?? "");
  const [ifMin, setIfMin] = useState<string>(jq.if_min != null ? String(jq.if_min) : "");
  const [jqSaving, setJqSaving] = useState(false);
  const [jqSaved, setJqSaved] = useState(false);
  const [jqExcluded, setJqExcluded] = useState<number | null>(null);
  const [reApplying, setReApplying] = useState(false);

  const [purifying, setPurifying] = useState(false);
  const [purifyResult, setPurifyResult] = useState<number | null>(null);

  const reload = async () => {
    const c = await api.listCriteria();
    setCriteria(c);
    setLoading(false);
  };

  useEffect(() => {
    reload();
  }, []);

  const startEdit = (c: ExclusionCriterionRow) => {
    setEditing(c.id);
    setDraft({ label: c.label, rationale: c.rationale ?? "" });
  };

  const saveEdit = async () => {
    if (!editing) return;
    await api.saveCriterion({
      id: editing,
      label: draft.label,
      rationale: draft.rationale || null,
      is_deferred: false,
    });
    setEditing(null);
    await reload();
    onChanged();
  };

  const addNew = async () => {
    if (!adding || !adding.label.trim()) return;
    await api.saveCriterion({
      label: adding.label,
      rationale: adding.rationale || null,
      is_deferred: false,
    });
    setAdding(null);
    await reload();
    onChanged();
  };

  const remove = async (id: number) => {
    await api.deleteCriterion(id);
    await reload();
    onChanged();
  };

  const saveJournalQuality = async () => {
    setJqSaving(true);
    setJqExcluded(null);
    const payload: { abs_min?: string; abdc_min?: string; if_min?: number } = {};
    if (absMin) payload.abs_min = absMin;
    if (abdcMin) payload.abdc_min = abdcMin;
    const ifParsed = parseFloat(ifMin);
    if (ifMin && !isNaN(ifParsed)) payload.if_min = ifParsed;

    await api.saveDecision({
      stage: "arranging",
      sub_stage: "criteria",
      decision_key: "project.journal_quality",
      value_json: JSON.stringify(JSON.stringify(payload)),
      rationale: null,
    });

    // Auto-exclude articles below the threshold
    const count = await api.applyQualityExclusions({
      abs_min: payload.abs_min,
      abdc_min: payload.abdc_min,
      if_min: payload.if_min,
    });

    setJqSaving(false);
    setJqSaved(true);
    setJqExcluded(count);
    setTimeout(() => { setJqSaved(false); setJqExcluded(null); }, 4000);
    onChanged();
  };

  const reApplyThresholds = async () => {
    setReApplying(true);
    setJqExcluded(null);
    const payload: { abs_min?: string; abdc_min?: string; if_min?: number } = {};
    if (absMin) payload.abs_min = absMin;
    if (abdcMin) payload.abdc_min = abdcMin;
    const ifParsed = parseFloat(ifMin);
    if (ifMin && !isNaN(ifParsed)) payload.if_min = ifParsed;

    const count = await api.applyQualityExclusions({
      abs_min: payload.abs_min,
      abdc_min: payload.abdc_min,
      if_min: payload.if_min,
    });

    setReApplying(false);
    setJqExcluded(count);
    setTimeout(() => setJqExcluded(null), 4000);
    onChanged();
  };

  const handlePurify = async () => {
    setPurifying(true);
    setPurifyResult(null);
    try {
      const count = await api.runPurification();
      setPurifyResult(count);
      onChanged();
      setTimeout(() => setPurifyResult(null), 5000);
    } finally {
      setPurifying(false);
    }
  };

  if (loading) {
    return <div className="text-[13px] text-[var(--color-paper-500)]">Loading criteria…</div>;
  }

  return (
    <div className="pl-6 pr-4 xl:pl-12 xl:pr-8">
      <div className="mb-6 flex items-end justify-between">
        <div>
          <div className="eyebrow text-[var(--color-paper-500)]">Purification</div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">
            Exclusion criteria
          </h2>
          <p className="text-[14px] text-[var(--color-paper-600)] mt-2 max-w-2xl leading-relaxed">
            The standard SPAR-4-SLR criteria are seeded for you. Edit any to fit your domain or
            add custom ones. Criterion #1 (Duplicate) is reserved and can't be deleted.
          </p>
        </div>
        <Button
          variant="secondary"
          size="sm"
          onClick={() => setAdding(adding ? null : { label: "", rationale: "" })}
        >
          <Plus size={13} strokeWidth={2} />
          Add criterion
        </Button>
      </div>

      {/* ── Run purification ── */}
      <div className="mb-6 p-4 rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)]">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h3 className="text-[14px] font-semibold text-[var(--color-ink-900)]">Run purification</h3>
            <p className="text-[12px] text-[var(--color-paper-600)] leading-relaxed mt-1 max-w-md">
              Apply the deterministic exclusion criteria (year outside the configured search period)
              in one click. Articles without a year are left alone.
            </p>
          </div>
          <button
            type="button"
            onClick={handlePurify}
            disabled={purifying}
            className="shrink-0 inline-flex items-center gap-1.5 px-3 py-2 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-accent-700)] border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)] transition-paper disabled:opacity-50 disabled:cursor-wait"
          >
            {purifying ? <Loader2 size={13} className="animate-spin" /> : <Filter size={13} strokeWidth={2} />}
            {purifying ? "Applying…" : "Run purification"}
          </button>
        </div>
        {purifyResult !== null && (
          <p className="mt-3 text-[12px] text-[var(--color-ok-700)]">
            ✓ Excluded {purifyResult} article{purifyResult === 1 ? "" : "s"} for falling outside the search period.
          </p>
        )}
      </div>

      {/* ── Journal quality threshold ── */}
      <Card className="p-5 mb-6 border-[var(--color-paper-300)]">
        <div className="flex items-start gap-3 mb-4">
          <div className="w-8 h-8 rounded-[var(--radius-sm)] bg-[var(--color-ink-100)] flex items-center justify-center shrink-0">
            <BookOpen size={14} strokeWidth={1.75} className="text-[var(--color-ink-700)]" />
          </div>
          <div>
            <h3 className="font-medium text-[14px] text-[var(--color-ink-900)]">
              Journal quality threshold
            </h3>
            <p className="text-[12px] text-[var(--color-paper-600)] mt-0.5 leading-relaxed max-w-xl">
              Articles in journals that don't meet the minimum rating will be automatically excluded.
              Leave a field blank to skip that threshold.
            </p>
          </div>
        </div>

        <div className="grid grid-cols-3 gap-4 max-w-2xl">
          <Select
            label="Minimum ABS rating"
            value={absMin}
            onChange={(e) => setAbsMin(e.target.value)}
            hint="ABS Academic Journal Guide"
          >
            <option value="">— no threshold —</option>
            {[...ABS_LEVELS].reverse().map((r) => (
              <option key={r} value={r}>{r} and above</option>
            ))}
          </Select>
          <Select
            label="Minimum ABDC rating"
            value={abdcMin}
            onChange={(e) => setAbdcMin(e.target.value)}
            hint="Australian Business Deans Council"
          >
            <option value="">— no threshold —</option>
            {[...ABDC_LEVELS].reverse().map((r) => (
              <option key={r} value={r}>{r} and above</option>
            ))}
          </Select>
          <TextField
            label="Minimum impact factor"
            value={ifMin}
            onChange={(e) => setIfMin(e.target.value)}
            placeholder="e.g. 2.5"
            hint="Leave blank for no IF threshold"
          />
        </div>

        <div className="mt-4 flex items-center gap-3 flex-wrap">
          <Button onClick={saveJournalQuality} disabled={jqSaving || reApplying} variant="primary" size="sm">
            <Save size={12} strokeWidth={2} />
            {jqSaving ? "Applying…" : "Save & apply"}
          </Button>
          <Button onClick={reApplyThresholds} disabled={jqSaving || reApplying} variant="secondary" size="sm">
            {reApplying
              ? <Loader2 size={12} className="animate-spin" />
              : <RefreshCw size={12} strokeWidth={2} />
            }
            Re-apply to articles
          </Button>
          {(jqSaved || jqExcluded !== null) && (
            <motion.span
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              className="text-[12px] text-[var(--color-ok-500)]"
            >
              {jqSaved && "Saved"}
              {jqExcluded !== null && jqExcluded > 0 && (
                <span className={jqSaved ? "ml-1 text-[var(--color-paper-500)]" : "text-[var(--color-paper-600)]"}>
                  {jqSaved ? "· " : ""}{jqExcluded} article{jqExcluded !== 1 ? "s" : ""} auto-excluded
                </span>
              )}
              {jqExcluded === 0 && (
                <span className={jqSaved ? "ml-1 text-[var(--color-paper-500)]" : "text-[var(--color-paper-600)]"}>
                  {jqSaved ? "· " : ""}no new exclusions
                </span>
              )}
            </motion.span>
          )}
        </div>
      </Card>

      {/* ── Exclusion criteria list ── */}
      <div className="space-y-2">
        {criteria.map((c) => (
          <motion.div key={c.id} layout>
            <Card className="p-4">
              {editing === c.id ? (
                <div className="space-y-3">
                  <TextField
                    label="Label"
                    value={draft.label}
                    onChange={(e) => setDraft({ ...draft, label: e.target.value })}
                  />
                  <TextArea
                    label="Rationale"
                    rows={2}
                    value={draft.rationale}
                    onChange={(e) => setDraft({ ...draft, rationale: e.target.value })}
                  />
                  <div className="flex gap-2">
                    <Button onClick={saveEdit} variant="primary" size="sm">
                      <Save size={12} strokeWidth={2} />
                      Save
                    </Button>
                    <Button onClick={() => setEditing(null)} variant="ghost" size="sm">
                      Cancel
                    </Button>
                  </div>
                </div>
              ) : (
                <div className="flex items-start gap-4 group">
                  <div className="font-mono text-[11px] tabular-nums text-[var(--color-paper-500)] mt-0.5 shrink-0 w-6 text-right">
                    #{c.number}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-[14px] text-[var(--color-ink-900)]">
                      {c.label}
                    </div>
                    {c.rationale && (
                      <div className="text-[12px] text-[var(--color-paper-600)] mt-0.5 leading-relaxed">
                        {c.rationale}
                      </div>
                    )}
                  </div>
                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-paper">
                    <button
                      onClick={() => startEdit(c)}
                      className="px-2 py-1 text-[11px] text-[var(--color-paper-600)] hover:text-[var(--color-ink-800)] rounded-sm hover:bg-[var(--color-paper-200)]"
                    >
                      Edit
                    </button>
                    {c.number > 1 && (
                      <button
                        onClick={() => remove(c.id)}
                        className="p-1.5 text-[var(--color-paper-500)] hover:text-[var(--color-error-500)] rounded-sm hover:bg-[var(--color-paper-200)]"
                      >
                        <Trash2 size={12} strokeWidth={1.75} />
                      </button>
                    )}
                  </div>
                </div>
              )}
            </Card>
          </motion.div>
        ))}
      </div>

      <AnimatePresence>
        {adding && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            className="overflow-hidden"
          >
            <Card className="p-5 mt-4">
              <div className="space-y-3">
                <TextField
                  label="New criterion label"
                  value={adding.label}
                  onChange={(e) => setAdding({ ...adding, label: e.target.value })}
                  placeholder="e.g., Pre-2010 publication"
                />
                <TextArea
                  label="Rationale"
                  rows={2}
                  value={adding.rationale}
                  onChange={(e) => setAdding({ ...adding, rationale: e.target.value })}
                  placeholder="Why this criterion applies to your review"
                />
                <div className="flex gap-2">
                  <Button onClick={addNew} variant="primary" size="sm">
                    Save
                  </Button>
                  <Button onClick={() => setAdding(null)} variant="ghost" size="sm">
                    <X size={13} />
                    Cancel
                  </Button>
                </div>
              </div>
            </Card>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
