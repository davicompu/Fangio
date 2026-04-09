import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Layers, Plus, Tag, Trash2, X } from "lucide-react";
import {
  api,
  type DimensionInfo,
  type FrameworkCodeDefinition,
} from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { TextField } from "../../components/ui/TextField";

interface Props {
  refreshKey: number;
  onChanged: () => void;
}

export function OrganizationPanel({ onChanged }: Props) {
  const [dimensions, setDimensions] = useState<DimensionInfo[]>([]);
  const [defs, setDefs] = useState<FrameworkCodeDefinition[]>([]);
  const [adding, setAdding] = useState<{ dim: string; code: string; desc: string } | null>(null);
  const [loading, setLoading] = useState(true);

  const reload = async () => {
    const [dims, codes] = await Promise.all([
      api.listFrameworkDimensions(),
      api.listFrameworkCodeDefs(),
    ]);
    setDimensions(dims);
    setDefs(codes);
    setLoading(false);
  };

  useEffect(() => {
    reload();
  }, []);

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

  if (loading) {
    return <div className="text-[13px] text-[var(--color-paper-500)]">Loading framework…</div>;
  }

  return (
    <div className="max-w-5xl">
      <div className="mb-8">
        <div className="eyebrow text-[var(--color-paper-500)] mb-1">Organising framework</div>
        <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)]">
          Define starter codes per dimension
        </h2>
        <p className="text-[14px] text-[var(--color-paper-600)] mt-2 max-w-2xl leading-relaxed">
          You don't have to predefine every code — you can add them on the fly during coding. But
          a starter vocabulary keeps coding decisions consistent across batches and gives the
          local model something to suggest from.
        </p>
      </div>

      <div className="space-y-5">
        {dimensions.map((dim) => {
          const dimDefs = defs.filter((d) => d.dimension === dim.name);
          return (
            <Card key={dim.name} className="p-5">
              <div className="flex items-start justify-between mb-4">
                <div className="flex items-start gap-3">
                  <div className="w-9 h-9 rounded-[var(--radius-sm)] bg-[var(--color-ink-100)] text-[var(--color-ink-700)] flex items-center justify-center shrink-0">
                    <Layers size={16} strokeWidth={1.75} />
                  </div>
                  <div>
                    <h3 className="font-serif text-[20px] tracking-tight text-[var(--color-ink-900)]">
                      {dim.name}
                    </h3>
                    <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed max-w-xl">
                      {dim.description}
                    </p>
                  </div>
                </div>
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
                  Add code
                </Button>
              </div>

              <div className="flex flex-wrap gap-2">
                {dimDefs.length === 0 && (
                  <span className="text-[12px] italic text-[var(--color-paper-500)]">
                    No starter codes yet — add a few or just code on the fly.
                  </span>
                )}
                {dimDefs.map((d) => (
                  <CodeChip key={d.id} def={d} onDelete={() => remove(d.id)} />
                ))}
              </div>

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
                      />
                      <TextField
                        label="Description (optional)"
                        value={adding.desc}
                        onChange={(e) => setAdding({ ...adding, desc: e.target.value })}
                        placeholder="What this code captures"
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
