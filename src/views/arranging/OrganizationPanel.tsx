import { useEffect, useMemo, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Layers, Pencil, Plus, Tag, Trash2, X } from "lucide-react";
import {
  api,
  type CodingRow,
  type DimensionInfo,
  type FrameworkCodeDefinition,
  type ProjectRow,
} from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { TextField } from "../../components/ui/TextField";

interface Props {
  project: ProjectRow;
  refreshKey: number;
  onChanged: () => void;
  onJumpToCode?: (dimension: string, code: string) => void;
}

export function OrganizationPanel({ project, onChanged, refreshKey, onJumpToCode }: Props) {
  const isCustom = project.framework === "Custom";

  const [dimensions, setDimensions] = useState<DimensionInfo[]>([]);
  const [defs, setDefs] = useState<FrameworkCodeDefinition[]>([]);
  const [codings, setCodings] = useState<CodingRow[]>([]);
  const [adding, setAdding] = useState<{ dim: string; code: string; desc: string } | null>(null);
  const [loading, setLoading] = useState(true);
  const [addingDim, setAddingDim] = useState(false);
  const [newDimName, setNewDimName] = useState("");
  // Dimension overrides: hidden (Set of original names) + renamed (Map original→display)
  const [hidden, setHidden] = useState<Set<string>>(new Set());
  const [renamed, setRenamed] = useState<Map<string, string>>(new Map());
  const [editingDim, setEditingDim] = useState<{ original: string; value: string } | null>(null);

  const reload = async () => {
    const [dims, codes, batch] = await Promise.all([
      api.listFrameworkDimensions(),
      api.listFrameworkCodeDefs(),
      api.listCodingBatch(0, 9999),
    ]);
    setDimensions(dims);
    setDefs(codes);
    setCodings(batch.codings);
    setLoading(false);
  };

  useEffect(() => {
    reload();
    try {
      const raw = project.dimension_overrides
        ? JSON.parse(project.dimension_overrides)
        : {};
      setHidden(new Set((raw.hidden ?? []) as string[]));
      setRenamed(new Map(Object.entries((raw.renamed ?? {}) as Record<string, string>)));
    } catch {}
  }, [refreshKey]);

  // Group applied codings by dimension → code → count
  const appliedByDim = useMemo(() => {
    const map = new Map<string, Map<string, number>>();
    for (const c of codings) {
      if (!map.has(c.dimension)) map.set(c.dimension, new Map());
      const codes = map.get(c.dimension)!;
      codes.set(c.code, (codes.get(c.code) ?? 0) + 1);
    }
    return map;
  }, [codings]);

  const saveCustomDimensions = async (names: string[]) => {
    await api.saveDecision({
      stage: "arranging",
      sub_stage: "organization",
      decision_key: "project.custom_dimensions",
      value_json: JSON.stringify(JSON.stringify(names)),
      rationale: null,
    });
    await reload();
    onChanged();
  };

  const add = async () => {
    if (!adding || !adding.code.trim()) return;
    await api.addFrameworkCodeDef(adding.dim, adding.code.trim(), adding.desc.trim() || undefined);
    setAdding(null);
    await reload();
    onChanged();
  };

  const remove = async (id: number) => {
    await api.deleteFrameworkCodeDef(id);
    await reload();
    onChanged();
  };

  const saveOverrides = async (nextHidden: Set<string>, nextRenamed: Map<string, string>) => {
    const payload = {
      hidden: Array.from(nextHidden),
      renamed: Object.fromEntries(nextRenamed),
    };
    await api.saveDecision({
      stage: "arranging",
      sub_stage: "organization",
      decision_key: "project.dimension_overrides",
      value_json: JSON.stringify(JSON.stringify(payload)),
      rationale: null,
    });
    await reload();
    onChanged();
  };

  const hideDimension = (original: string) => {
    const next = new Set(hidden);
    next.add(original);
    setHidden(next);
    saveOverrides(next, renamed);
  };

  const renameDimension = (original: string, newName: string) => {
    const next = new Map(renamed);
    if (newName.trim() && newName.trim() !== original) {
      next.set(original, newName.trim());
    } else {
      next.delete(original);
    }
    setRenamed(next);
    setEditingDim(null);
    saveOverrides(hidden, next);
  };

  const handleAddDimension = async () => {
    const name = newDimName.trim();
    if (!name) return;
    setNewDimName("");
    setAddingDim(false);
    const exists = dimensions.some((d) => d.name.toLowerCase() === name.toLowerCase());
    if (exists) return;
    const current: string[] = project.custom_dimensions
      ? JSON.parse(project.custom_dimensions)
      : dimensions.map((d) => d.name);
    await saveCustomDimensions([...current, name]);
  };

  const removeCustomDimension = async (name: string) => {
    const current: string[] = project.custom_dimensions
      ? JSON.parse(project.custom_dimensions)
      : dimensions.map((d) => d.name);
    await saveCustomDimensions(current.filter((d) => d !== name));
  };

  if (loading) {
    return <div className="text-[13px] text-[var(--color-paper-500)]">Loading framework…</div>;
  }

  const allDims = dimensions;

  return (
    <div className="pl-6 pr-4 xl:pl-12 xl:pr-8">
      <div className="mb-8 flex items-start justify-between">
        <div>
          <div className="eyebrow text-[var(--color-paper-500)] mb-1">
            Organising framework · {project.framework}
          </div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)]">
            Dimensions & codes
          </h2>
          <p className="text-[14px] text-[var(--color-paper-600)] mt-2 max-w-2xl leading-relaxed">
            {isCustom
              ? "Define your dimensions, add starter codes as a reference vocabulary, and see every code applied so far."
              : "Add starter codes as a reference vocabulary per dimension, and see every code applied so far. You can also rename or remove dimensions you won't use."}
          </p>
        </div>
        {isCustom && (
          <div className="shrink-0 mt-1">
            {addingDim ? (
              <div className="flex items-center gap-2">
                <input
                  autoFocus
                  value={newDimName}
                  onChange={(e) => setNewDimName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleAddDimension();
                    if (e.key === "Escape") { setAddingDim(false); setNewDimName(""); }
                  }}
                  placeholder="Dimension name…"
                  className="h-8 px-3 text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-ink-400)] bg-white text-[var(--color-ink-800)] focus:outline-none w-52"
                />
                <button
                  type="button"
                  onClick={handleAddDimension}
                  className="px-3 h-8 text-[12px] font-medium rounded-[var(--radius-xs)] bg-[var(--color-ink-700)] text-white hover:bg-[var(--color-ink-900)] transition-paper"
                >
                  Add
                </button>
                <button
                  type="button"
                  onClick={() => { setAddingDim(false); setNewDimName(""); }}
                  className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
                >
                  <X size={13} strokeWidth={2} className="text-[var(--color-paper-500)]" />
                </button>
              </div>
            ) : (
              <Button variant="ghost" size="sm" onClick={() => setAddingDim(true)}>
                <Plus size={13} strokeWidth={2} />
                Add dimension
              </Button>
            )}
          </div>
        )}
      </div>

      {isCustom && dimensions.length === 0 && (
        <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-12 text-center mb-6">
          <Layers size={24} strokeWidth={1.5} className="text-[var(--color-paper-300)] mx-auto mb-3" />
          <p className="text-[14px] text-[var(--color-paper-700)] font-medium mb-1">No dimensions yet</p>
          <p className="text-[13px] text-[var(--color-paper-500)] max-w-sm mx-auto leading-relaxed mb-4">
            Click <strong>Add dimension</strong> to define the first dimension — for example
            "Theoretical lens", "Industry", or "Methodology".
          </p>
          <Button variant="primary" size="md" onClick={() => setAddingDim(true)}>
            <Plus size={13} strokeWidth={2} />
            Add your first dimension
          </Button>
        </div>
      )}

      <div className="space-y-5">
        {allDims.map((dim) => {
          const originalName =
            Array.from(renamed.entries()).find(([, v]) => v === dim.name)?.[0] ?? dim.name;
          const dimDefs = defs.filter(
            (d) => d.dimension === originalName || d.dimension === dim.name,
          );
          const appliedCodes = appliedByDim.get(originalName) ?? appliedByDim.get(dim.name) ?? new Map<string, number>();
          const isEditingThis = editingDim?.original === originalName;

          return (
            <Card key={dim.name} className="p-5">
              {/* ── Card header ── */}
              <div className="flex items-start justify-between mb-5">
                <div className="flex items-start gap-3">
                  <div className="w-9 h-9 rounded-[var(--radius-sm)] bg-[var(--color-ink-100)] text-[var(--color-ink-700)] flex items-center justify-center shrink-0">
                    <Layers size={16} strokeWidth={1.75} />
                  </div>
                  <div>
                    {isEditingThis ? (
                      <div className="flex items-center gap-2">
                        <input
                          autoFocus
                          value={editingDim!.value}
                          onChange={(e) =>
                            setEditingDim({ original: originalName, value: e.target.value })
                          }
                          onKeyDown={(e) => {
                            if (e.key === "Enter") renameDimension(originalName, editingDim!.value);
                            if (e.key === "Escape") setEditingDim(null);
                          }}
                          className="h-8 px-3 font-serif text-[18px] rounded-[var(--radius-xs)] border border-[var(--color-ink-400)] bg-white focus:outline-none w-60"
                        />
                        <button
                          type="button"
                          onClick={() => renameDimension(originalName, editingDim!.value)}
                          className="px-3 h-8 text-[12px] font-medium rounded-[var(--radius-xs)] bg-[var(--color-ink-700)] text-white hover:bg-[var(--color-ink-900)] transition-paper"
                        >
                          Save
                        </button>
                        <button
                          type="button"
                          onClick={() => setEditingDim(null)}
                          className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
                        >
                          <X size={13} strokeWidth={2} className="text-[var(--color-paper-500)]" />
                        </button>
                      </div>
                    ) : (
                      <h3 className="font-serif text-[20px] tracking-tight text-[var(--color-ink-900)]">
                        {dim.name}
                        {renamed.has(originalName) && (
                          <span className="ml-2 text-[12px] font-sans font-normal italic text-[var(--color-paper-400)]">
                            (originally {originalName})
                          </span>
                        )}
                      </h3>
                    )}
                    {dim.description ? (
                      <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed max-w-xl mt-0.5">
                        {dim.description}
                      </p>
                    ) : null}
                  </div>
                </div>

                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setEditingDim({ original: originalName, value: dim.name })}
                  >
                    <Pencil size={12} strokeWidth={1.75} />
                    Rename
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() =>
                      isCustom ? removeCustomDimension(originalName) : hideDimension(originalName)
                    }
                  >
                    <Trash2 size={12} strokeWidth={1.75} />
                    Remove
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() =>
                      setAdding(
                        adding?.dim === dim.name
                          ? null
                          : { dim: dim.name, code: "", desc: "" },
                      )
                    }
                  >
                    <Plus size={13} strokeWidth={2} />
                    Add starter code
                  </Button>
                </div>
              </div>

              {/* ── Starter codes ── */}
              <div className="mb-1">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-paper-400)] mb-2">
                  Starter codes
                </div>
                <div className="flex flex-wrap gap-2 min-h-[26px]">
                  {dimDefs.length === 0 && (
                    <span className="text-[12px] italic text-[var(--color-paper-400)]">
                      None yet — add a few or just code on the fly.
                    </span>
                  )}
                  {dimDefs.map((d) => (
                    <CodeChip key={d.id} def={d} onDelete={() => remove(d.id)} />
                  ))}
                </div>
              </div>

              {/* ── Applied codes ── */}
              {appliedCodes.size > 0 && (
                <div className="mt-4 pt-4 border-t border-[var(--color-paper-200)]">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-paper-400)] mb-2">
                    Applied in coding
                    <span className="ml-1.5 font-mono normal-case tracking-normal text-[var(--color-paper-300)]">
                      · {appliedCodes.size} unique
                    </span>
                    {onJumpToCode && (
                      <span className="ml-1.5 font-sans normal-case tracking-normal text-[var(--color-paper-400)] italic">
                        · click to filter in Coding
                      </span>
                    )}
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    {Array.from(appliedCodes.entries())
                      .sort((a, b) => b[1] - a[1])
                      .map(([code, count]) => (
                        onJumpToCode ? (
                          <button
                            key={code}
                            onClick={() => onJumpToCode(originalName, code)}
                            className="inline-flex items-center gap-1.5 pl-2.5 pr-2 py-0.5 rounded-full bg-[var(--color-ok-50)] border border-[var(--color-ok-200)] text-[11px] text-[var(--color-ok-700)] hover:bg-[var(--color-ok-100)] hover:border-[var(--color-ok-400)] transition-paper"
                          >
                            <Tag size={9} strokeWidth={2} />
                            <span className="font-medium">{code}</span>
                            <span className="font-mono text-[10px] text-[var(--color-ok-500)]">×{count}</span>
                          </button>
                        ) : (
                          <span
                            key={code}
                            className="inline-flex items-center gap-1.5 pl-2.5 pr-2 py-0.5 rounded-full bg-[var(--color-ok-50)] border border-[var(--color-ok-200)] text-[11px] text-[var(--color-ok-700)]"
                          >
                            <Tag size={9} strokeWidth={2} />
                            <span className="font-medium">{code}</span>
                            <span className="font-mono text-[10px] text-[var(--color-ok-500)]">×{count}</span>
                          </span>
                        )
                      ))}
                  </div>
                </div>
              )}

              {/* ── Add starter code form ── */}
              <AnimatePresence>
                {adding?.dim === dim.name && (
                  <motion.div
                    initial={{ opacity: 0, height: 0 }}
                    animate={{ opacity: 1, height: "auto" }}
                    exit={{ opacity: 0, height: 0 }}
                    className="overflow-hidden"
                  >
                    <div className="mt-4 pt-4 border-t border-[var(--color-paper-200)] grid grid-cols-[1fr_2fr_auto] gap-3">
                      <TextField
                        label="Code"
                        value={adding.code}
                        onChange={(e) => setAdding({ ...adding, code: e.target.value })}
                        placeholder="e.g., Trust"
                        onKeyDown={(e: React.KeyboardEvent) => {
                          if (e.key === "Enter") add();
                        }}
                      />
                      <TextField
                        label="Description (optional)"
                        value={adding.desc}
                        onChange={(e) => setAdding({ ...adding, desc: e.target.value })}
                        placeholder="What this code captures"
                        onKeyDown={(e: React.KeyboardEvent) => {
                          if (e.key === "Enter") add();
                        }}
                      />
                      <div className="flex items-end gap-1">
                        <Button onClick={add} variant="primary" size="md">
                          Save
                        </Button>
                        <Button onClick={() => setAdding(null)} variant="ghost" size="md">
                          <X size={14} />
                        </Button>
                      </div>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </Card>
          );
        })}
      </div>
    </div>
  );
}

function CodeChip({
  def,
  onDelete,
}: {
  def: FrameworkCodeDefinition;
  onDelete: () => void;
}) {
  return (
    <motion.div
      layout
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.9 }}
      className="group inline-flex items-center gap-1.5 pl-2.5 pr-1.5 py-1 rounded-full bg-[var(--color-paper-100)] border border-[var(--color-paper-300)]"
    >
      <Tag size={11} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
      <span className="text-[12px] font-medium text-[var(--color-ink-800)]">{def.code}</span>
      {def.description && (
        <span className="text-[11px] text-[var(--color-paper-500)] italic max-w-[200px] truncate">
          · {def.description}
        </span>
      )}
      <button
        onClick={onDelete}
        className="opacity-0 group-hover:opacity-100 ml-0.5 p-0.5 rounded-sm hover:bg-[var(--color-paper-300)] transition-paper"
        title="Remove"
      >
        <Trash2 size={11} strokeWidth={1.75} className="text-[var(--color-paper-600)]" />
      </button>
    </motion.div>
  );
}
