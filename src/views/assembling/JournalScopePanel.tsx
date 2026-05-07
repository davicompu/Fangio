import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { BookOpen, Check, ChevronDown, ChevronUp, Pencil, Plus, Star, Trash2, X } from "lucide-react";
import { api, type JournalRow, type JournalInput } from "../../lib/api";
import { Button } from "../../components/ui/Button";
import { TextField } from "../../components/ui/TextField";
import { MarkStepComplete } from "../../components/MarkStepComplete";

const ABS_RATINGS = ["4*", "4", "3", "2", "1"] as const;
const ABDC_RATINGS = ["A*", "A", "B", "C"] as const;

const FIELDS = [
  "Management",
  "Finance",
  "Accounting",
  "Marketing",
  "Information Systems",
  "Operations",
  "HRM / OB",
  "Entrepreneurship",
  "Innovation",
  "International Business",
  "Other",
];

function absColor(rating: string | null) {
  if (!rating) return "text-[var(--color-paper-400)]";
  if (rating === "4*") return "text-[var(--color-ok-600)] font-semibold";
  if (rating === "4") return "text-[var(--color-ok-500)] font-medium";
  if (rating === "3") return "text-[var(--color-ink-700)]";
  return "text-[var(--color-paper-500)]";
}

function abdcColor(rating: string | null) {
  if (!rating) return "text-[var(--color-paper-400)]";
  if (rating === "A*") return "text-[var(--color-ok-600)] font-semibold";
  if (rating === "A") return "text-[var(--color-ok-500)] font-medium";
  return "text-[var(--color-paper-500)]";
}

interface EditFormProps {
  initial?: JournalRow;
  onSave: (input: JournalInput) => Promise<void>;
  onCancel: () => void;
}

function EditForm({ initial, onSave, onCancel }: EditFormProps) {
  const [name, setName] = useState(initial?.name ?? "");
  const [issn, setIssn] = useState(initial?.issn ?? "");
  const [absRating, setAbsRating] = useState(initial?.abs_rating ?? "");
  const [abdcRating, setAbdcRating] = useState(initial?.abdc_rating ?? "");
  const [impact, setImpact] = useState(initial?.impact_factor?.toString() ?? "");
  const [field, setField] = useState(initial?.field ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSave = async () => {
    if (!name.trim()) { setError("Journal name is required."); return; }
    setBusy(true);
    setError(null);
    try {
      await onSave({
        id: initial?.id ?? null,
        name: name.trim(),
        issn: issn.trim() || null,
        abs_rating: absRating || null,
        abdc_rating: abdcRating || null,
        impact_factor: impact ? parseFloat(impact) : null,
        field: field.trim() || null,
      });
    } catch (e: any) {
      setError(e?.message ?? "Save failed.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: -6 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -6 }}
      className="bg-[var(--color-paper-50)] border border-[var(--color-paper-300)] rounded-[var(--radius-sm)] p-5 space-y-4"
    >
      <p className="text-[13px] font-medium text-[var(--color-ink-800)]">
        {initial ? "Edit journal" : "Add journal"}
      </p>

      <TextField
        label="Journal name"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="e.g., Journal of Business Venturing"
      />

      <div className="grid grid-cols-2 gap-4">
        <TextField
          label="ISSN (optional)"
          value={issn}
          onChange={(e) => setIssn(e.target.value)}
          placeholder="e.g., 0883-9026"
          mono
        />
        <TextField
          label="Impact factor (approx.)"
          value={impact}
          onChange={(e) => setImpact(e.target.value)}
          placeholder="e.g., 9.0"
          mono
        />
      </div>

      <div className="grid grid-cols-3 gap-4">
        {/* ABS rating */}
        <div>
          <label className="block text-[11px] font-medium text-[var(--color-paper-600)] mb-1.5 uppercase tracking-wider">
            ABS rating
          </label>
          <div className="flex flex-wrap gap-1.5">
            {["", ...ABS_RATINGS].map((r) => (
              <button
                key={r || "none"}
                type="button"
                onClick={() => setAbsRating(r)}
                className={
                  "px-2 py-0.5 text-[11px] rounded-[3px] border transition-paper " +
                  (absRating === r
                    ? "bg-[var(--color-ink-700)] text-white border-[var(--color-ink-700)]"
                    : "border-[var(--color-paper-300)] hover:border-[var(--color-paper-500)]")
                }
              >
                {r || "—"}
              </button>
            ))}
          </div>
        </div>

        {/* ABDC rating */}
        <div>
          <label className="block text-[11px] font-medium text-[var(--color-paper-600)] mb-1.5 uppercase tracking-wider">
            ABDC rating
          </label>
          <div className="flex flex-wrap gap-1.5">
            {["", ...ABDC_RATINGS].map((r) => (
              <button
                key={r || "none"}
                type="button"
                onClick={() => setAbdcRating(r)}
                className={
                  "px-2 py-0.5 text-[11px] rounded-[3px] border transition-paper " +
                  (abdcRating === r
                    ? "bg-[var(--color-ink-700)] text-white border-[var(--color-ink-700)]"
                    : "border-[var(--color-paper-300)] hover:border-[var(--color-paper-500)]")
                }
              >
                {r || "—"}
              </button>
            ))}
          </div>
        </div>

        {/* Field */}
        <div>
          <label className="block text-[11px] font-medium text-[var(--color-paper-600)] mb-1.5 uppercase tracking-wider">
            Field
          </label>
          <select
            value={field}
            onChange={(e) => setField(e.target.value)}
            className="w-full h-8 px-2.5 text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white text-[var(--color-ink-800)] focus:outline-none focus:border-[var(--color-ink-600)]"
          >
            <option value="">— Any —</option>
            {FIELDS.map((f) => (
              <option key={f} value={f}>{f}</option>
            ))}
          </select>
        </div>
      </div>

      {error && <p className="text-[12px] text-[var(--color-warn-600)]">{error}</p>}

      <div className="flex items-center gap-2 pt-1">
        <Button onClick={handleSave} disabled={busy} variant="primary" size="sm">
          <Check size={12} strokeWidth={2.5} />
          {busy ? "Saving…" : "Save"}
        </Button>
        <Button onClick={onCancel} disabled={busy} variant="ghost" size="sm">
          Cancel
        </Button>
      </div>
    </motion.div>
  );
}

export function JournalScopePanel({
  manualComplete,
  onChanged,
}: {
  manualComplete?: boolean;
  onChanged?: () => void;
}) {
  const [journals, setJournals] = useState<JournalRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [editingId, setEditingId] = useState<number | "new" | null>(null);
  const [filterField, setFilterField] = useState<string>("");
  const [filterAbs, setFilterAbs] = useState<string>("");
  const [filterAbdc, setFilterAbdc] = useState<string>("");
  const [expandedFields, setExpandedFields] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    try {
      const rows = await api.listJournals();
      setJournals(rows);
    } catch (e: any) {
      setError(e?.message ?? "Failed to load journals.");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const handleSave = async (input: JournalInput) => {
    await api.saveJournal(input);
    setEditingId(null);
    await load();
  };

  const handleDelete = async (id: number) => {
    await api.deleteJournal(id);
    await load();
  };

  const toggleField = (field: string) => {
    setExpandedFields((prev) => {
      const next = new Set(prev);
      if (next.has(field)) next.delete(field);
      else next.add(field);
      return next;
    });
  };

  // Apply filters
  const filtered = journals.filter((j) => {
    if (filterField && j.field !== filterField) return false;
    if (filterAbs && j.abs_rating !== filterAbs) return false;
    if (filterAbdc && j.abdc_rating !== filterAbdc) return false;
    return true;
  });

  // Group by field
  const byField = filtered.reduce<Record<string, JournalRow[]>>((acc, j) => {
    const f = j.field ?? "Other";
    if (!acc[f]) acc[f] = [];
    acc[f].push(j);
    return acc;
  }, {});
  const fieldKeys = Object.keys(byField).sort();

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <span className="eyebrow text-[var(--color-paper-400)]">Loading journals…</span>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <div className="flex items-center gap-2 mb-1.5 text-[var(--color-paper-500)]">
            <BookOpen size={14} strokeWidth={1.75} />
            <span className="eyebrow">Journal scope</span>
          </div>
          <p className="text-[14px] text-[var(--color-paper-600)] leading-relaxed max-w-2xl">
            A curated list of peer-reviewed journals for scoping your review. Seeded with ABS and
            ABDC ratings for high-impact business journals — add, edit, or remove as needed. PRISMA
            requires you to document any journal restrictions in your search strategy.
          </p>
        </div>
        <Button
          variant="primary"
          size="sm"
          onClick={() => setEditingId("new")}
          className="shrink-0 ml-6"
        >
          <Plus size={13} strokeWidth={2} />
          Add journal
        </Button>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-3 flex-wrap">
        <select
          value={filterField}
          onChange={(e) => setFilterField(e.target.value)}
          className="h-8 px-2.5 text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white text-[var(--color-ink-800)] focus:outline-none focus:border-[var(--color-ink-600)]"
        >
          <option value="">All fields</option>
          {FIELDS.map((f) => <option key={f} value={f}>{f}</option>)}
        </select>
        <select
          value={filterAbs}
          onChange={(e) => setFilterAbs(e.target.value)}
          className="h-8 px-2.5 text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white text-[var(--color-ink-800)] focus:outline-none focus:border-[var(--color-ink-600)]"
        >
          <option value="">All ABS</option>
          {ABS_RATINGS.map((r) => <option key={r} value={r}>ABS {r}</option>)}
        </select>
        <select
          value={filterAbdc}
          onChange={(e) => setFilterAbdc(e.target.value)}
          className="h-8 px-2.5 text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white text-[var(--color-ink-800)] focus:outline-none focus:border-[var(--color-ink-600)]"
        >
          <option value="">All ABDC</option>
          {ABDC_RATINGS.map((r) => <option key={r} value={r}>ABDC {r}</option>)}
        </select>
        <span className="text-[12px] text-[var(--color-paper-500)]">
          {filtered.length} journal{filtered.length !== 1 ? "s" : ""}
        </span>
        {(filterField || filterAbs || filterAbdc) && (
          <button
            type="button"
            onClick={() => { setFilterField(""); setFilterAbs(""); setFilterAbdc(""); }}
            className="flex items-center gap-1 text-[12px] text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] transition-paper"
          >
            <X size={11} strokeWidth={2} />
            Clear filters
          </button>
        )}
      </div>

      {/* New journal form */}
      <AnimatePresence>
        {editingId === "new" && (
          <EditForm
            onSave={handleSave}
            onCancel={() => setEditingId(null)}
          />
        )}
      </AnimatePresence>

      {error && (
        <p className="text-[13px] text-[var(--color-warn-600)]">{error}</p>
      )}

      {/* Grouped table */}
      {fieldKeys.length === 0 ? (
        <div className="py-16 text-center">
          <p className="text-[13px] text-[var(--color-paper-400)]">No journals match your filters.</p>
        </div>
      ) : (
        <div className="space-y-4">
          {fieldKeys.map((field) => {
            const items = byField[field];
            const expanded = expandedFields.has(field);
            return (
              <div key={field} className="border border-[var(--color-paper-200)] rounded-[var(--radius-sm)] overflow-hidden">
                {/* Field group header */}
                <button
                  type="button"
                  onClick={() => toggleField(field)}
                  className="w-full flex items-center justify-between px-4 py-3 bg-[var(--color-paper-100)] hover:bg-[var(--color-paper-150)] transition-paper"
                >
                  <div className="flex items-center gap-3">
                    <span className="font-medium text-[13px] text-[var(--color-ink-800)]">{field}</span>
                    <span className="text-[11px] text-[var(--color-paper-500)]">
                      {items.length} journal{items.length !== 1 ? "s" : ""}
                    </span>
                  </div>
                  {expanded ? (
                    <ChevronUp size={14} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
                  ) : (
                    <ChevronDown size={14} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
                  )}
                </button>

                <AnimatePresence initial={false}>
                  {expanded && (
                    <motion.div
                      initial={{ height: 0 }}
                      animate={{ height: "auto" }}
                      exit={{ height: 0 }}
                      transition={{ duration: 0.2, ease: [0.2, 0.7, 0.2, 1] }}
                      style={{ overflow: "hidden" }}
                    >
                      <table className="w-full text-[12px]">
                        <thead>
                          <tr className="border-b border-[var(--color-paper-200)] bg-[var(--color-paper-50)]">
                            <th className="text-left px-4 py-2 font-medium text-[var(--color-paper-500)] w-[40%]">Journal</th>
                            <th className="text-left px-3 py-2 font-medium text-[var(--color-paper-500)]">ABS</th>
                            <th className="text-left px-3 py-2 font-medium text-[var(--color-paper-500)]">ABDC</th>
                            <th className="text-left px-3 py-2 font-medium text-[var(--color-paper-500)]">IF (approx.)</th>
                            <th className="text-left px-3 py-2 font-medium text-[var(--color-paper-500)]">ISSN</th>
                            <th className="px-3 py-2 w-[72px]" />
                          </tr>
                        </thead>
                        <tbody>
                          {items.map((j, idx) => (
                            <>
                              <tr
                                key={j.id}
                                className={
                                  "border-b border-[var(--color-paper-100)] last:border-b-0 " +
                                  (idx % 2 === 0 ? "bg-white" : "bg-[var(--color-paper-50)]")
                                }
                              >
                                <td className="px-4 py-2.5">
                                  <div className="flex items-center gap-1.5">
                                    {j.is_custom && (
                                      <Star size={10} strokeWidth={2} className="text-[var(--color-accent-400)] shrink-0" fill="currentColor" />
                                    )}
                                    <span className="text-[var(--color-ink-800)]">{j.name}</span>
                                  </div>
                                </td>
                                <td className={`px-3 py-2.5 ${absColor(j.abs_rating)}`}>
                                  {j.abs_rating ?? "—"}
                                </td>
                                <td className={`px-3 py-2.5 ${abdcColor(j.abdc_rating)}`}>
                                  {j.abdc_rating ?? "—"}
                                </td>
                                <td className="px-3 py-2.5 tabular-nums text-[var(--color-paper-600)]">
                                  {j.impact_factor != null ? j.impact_factor.toFixed(1) : "—"}
                                </td>
                                <td className="px-3 py-2.5 font-mono text-[11px] text-[var(--color-paper-500)]">
                                  {j.issn ?? "—"}
                                </td>
                                <td className="px-3 py-2.5">
                                  <div className="flex items-center gap-1 justify-end">
                                    <button
                                      type="button"
                                      onClick={() => setEditingId(editingId === j.id ? null : j.id)}
                                      className="p-1 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
                                      aria-label="Edit"
                                    >
                                      <Pencil size={11} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
                                    </button>
                                    <button
                                      type="button"
                                      onClick={() => handleDelete(j.id)}
                                      className="p-1 rounded-[var(--radius-xs)] hover:bg-[var(--color-warn-50)] transition-paper"
                                      aria-label="Delete"
                                    >
                                      <Trash2 size={11} strokeWidth={1.75} className="text-[var(--color-paper-400)] hover:text-[var(--color-warn-500)]" />
                                    </button>
                                  </div>
                                </td>
                              </tr>
                              <AnimatePresence>
                                {editingId === j.id && (
                                  <tr key={`edit-${j.id}`}>
                                    <td colSpan={6} className="p-3 bg-[var(--color-paper-50)]">
                                      <EditForm
                                        initial={j}
                                        onSave={handleSave}
                                        onCancel={() => setEditingId(null)}
                                      />
                                    </td>
                                  </tr>
                                )}
                              </AnimatePresence>
                            </>
                          ))}
                        </tbody>
                      </table>
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>
            );
          })}
        </div>
      )}

      <div className="text-[11px] text-[var(--color-paper-400)] leading-relaxed max-w-2xl space-y-1.5">
        <p>
          <strong className="text-[var(--color-paper-500)]">ABS</strong> — Chartered Association of Business Schools Academic Journal Guide (AJG).
          Scale: 4* (world elite) → 4 → 3 → 2 → 1.
        </p>
        <p>
          <strong className="text-[var(--color-paper-500)]">ABDC</strong> — Australian Business Deans Council Journal Quality List.
          Scale: A* (top) → A → B → C.
        </p>
        <p>
          <strong className="text-[var(--color-paper-500)]">Impact factor</strong> — approximate Journal Impact Factor (Clarivate/Web of Science) as bundled in Reeves's default list at the time of the last release. These figures are <em>not</em> updated live; always verify current values directly in Web of Science or Journal Citation Reports before citing them in your methods section.
        </p>
        <p>Custom entries (starred) are stored only in this project and not verified against any external source.</p>
      </div>
      <MarkStepComplete
        stage="assembling"
        tab="journals"
        isComplete={!!manualComplete}
        onChanged={onChanged ?? (() => {})}
      />
    </div>
  );
}
