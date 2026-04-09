import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Plus, Save, Trash2, X } from "lucide-react";
import { api, type ExclusionCriterionRow } from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { TextField, TextArea } from "../../components/ui/TextField";

interface Props {
  refreshKey: number;
  onChanged: () => void;
}

export function CriteriaPanel({ onChanged }: Props) {
  const [criteria, setCriteria] = useState<ExclusionCriterionRow[]>([]);
  const [adding, setAdding] = useState<{ label: string; rationale: string } | null>(null);
  const [editing, setEditing] = useState<number | null>(null);
  const [draft, setDraft] = useState<{ label: string; rationale: string }>({
    label: "",
    rationale: "",
  });
  const [loading, setLoading] = useState(true);

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

  if (loading) {
    return <div className="text-[13px] text-[var(--color-paper-500)]">Loading criteria…</div>;
  }

  return (
    <div className="max-w-4xl">
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
