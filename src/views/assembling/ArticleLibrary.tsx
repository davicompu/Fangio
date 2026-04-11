import { useEffect, useMemo, useRef, useState } from "react";
import {
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
  type ColumnDef,
  type SortingState,
} from "@tanstack/react-table";
import { AnimatePresence, motion } from "framer-motion";
import { ArrowUpDown, Check, ExternalLink, Library, Loader2, Search, Star, X } from "lucide-react";
import { api, type ArticleRow, type JournalRow } from "../../lib/api";
import { cn } from "../../lib/utils";

interface Props {
  refreshKey: number;
  onChanged: () => void;
}

const ABS_RATINGS = ["4*", "4", "3", "2", "1"] as const;
const ABDC_RATINGS = ["A*", "A", "B", "C"] as const;

function normalizeJournal(name: string) {
  return name.toLowerCase().replace(/^the\s+/, "").trim();
}

// ── Quick-rate modal ──────────────────────────────────────────────────────────

function QuickRateModal({
  journalName,
  existing,
  onSave,
  onClose,
}: {
  journalName: string;
  existing: JournalRow | null;
  onSave: (row: JournalRow) => void;
  onClose: () => void;
}) {
  const [absRating, setAbsRating] = useState(existing?.abs_rating ?? "");
  const [abdcRating, setAbdcRating] = useState(existing?.abdc_rating ?? "");
  const [impact, setImpact] = useState(existing?.impact_factor?.toString() ?? "");
  const [busy, setBusy] = useState(false);
  const overlayRef = useRef<HTMLDivElement>(null);

  const handleSave = async () => {
    setBusy(true);
    try {
      const id = await api.saveJournal({
        id: existing?.id ?? null,
        name: journalName,
        issn: existing?.issn ?? null,
        abs_rating: absRating || null,
        abdc_rating: abdcRating || null,
        impact_factor: impact ? parseFloat(impact) : null,
        field: existing?.field ?? null,
      });
      onSave({
        id,
        name: journalName,
        issn: existing?.issn ?? null,
        abs_rating: absRating || null,
        abdc_rating: abdcRating || null,
        impact_factor: impact ? parseFloat(impact) : null,
        field: existing?.field ?? null,
        is_custom: true,
        created_at: existing?.created_at ?? new Date().toISOString(),
      });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      ref={overlayRef}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/20"
      onMouseDown={(e) => { if (e.target === overlayRef.current) onClose(); }}
    >
      <motion.div
        initial={{ opacity: 0, scale: 0.97, y: 6 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.97, y: 6 }}
        transition={{ duration: 0.15, ease: [0.2, 0.7, 0.2, 1] }}
        className="bg-white border border-[var(--color-paper-300)] rounded-[var(--radius-md)] shadow-xl p-6 w-[460px] max-w-[95vw]"
      >
        <div className="flex items-start justify-between mb-5">
          <div className="flex-1 min-w-0 pr-3">
            <p className="eyebrow text-[var(--color-paper-500)] mb-1">Rate journal</p>
            <h3 className="font-serif text-[17px] leading-snug text-[var(--color-ink-900)] line-clamp-2">
              {journalName}
            </h3>
            {existing && (
              <span className="inline-flex items-center gap-1 text-[10px] text-[var(--color-accent-500)] mt-1">
                <Star size={9} strokeWidth={2} fill="currentColor" />
                Already in journal scope
              </span>
            )}
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] text-[var(--color-paper-500)] transition-paper shrink-0"
          >
            <X size={14} strokeWidth={2} />
          </button>
        </div>

        <div className="space-y-4">
          {/* ABS */}
          <div>
            <label className="block text-[11px] font-medium text-[var(--color-paper-600)] mb-2 uppercase tracking-wider">
              ABS rating
            </label>
            <div className="flex flex-wrap gap-1.5">
              {["", ...ABS_RATINGS].map((r) => (
                <button
                  key={r || "none"}
                  type="button"
                  onClick={() => setAbsRating(r)}
                  className={cn(
                    "px-2.5 py-1 text-[12px] rounded-[var(--radius-xs)] border transition-paper",
                    absRating === r
                      ? "bg-[var(--color-ink-700)] text-white border-[var(--color-ink-700)]"
                      : "border-[var(--color-paper-300)] text-[var(--color-paper-700)] hover:border-[var(--color-paper-500)]"
                  )}
                >
                  {r || "— none —"}
                </button>
              ))}
            </div>
          </div>

          {/* ABDC */}
          <div>
            <label className="block text-[11px] font-medium text-[var(--color-paper-600)] mb-2 uppercase tracking-wider">
              ABDC rating
            </label>
            <div className="flex flex-wrap gap-1.5">
              {["", ...ABDC_RATINGS].map((r) => (
                <button
                  key={r || "none"}
                  type="button"
                  onClick={() => setAbdcRating(r)}
                  className={cn(
                    "px-2.5 py-1 text-[12px] rounded-[var(--radius-xs)] border transition-paper",
                    abdcRating === r
                      ? "bg-[var(--color-ink-700)] text-white border-[var(--color-ink-700)]"
                      : "border-[var(--color-paper-300)] text-[var(--color-paper-700)] hover:border-[var(--color-paper-500)]"
                  )}
                >
                  {r || "— none —"}
                </button>
              ))}
            </div>
          </div>

          {/* Impact factor */}
          <div>
            <label className="block text-[11px] font-medium text-[var(--color-paper-600)] mb-2 uppercase tracking-wider">
              Impact factor (approx.)
            </label>
            <input
              type="number"
              step="0.01"
              min="0"
              value={impact}
              onChange={(e) => setImpact(e.target.value)}
              placeholder="e.g. 9.0"
              className="w-32 h-8 px-2.5 font-mono text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white text-[var(--color-ink-800)] placeholder:text-[var(--color-paper-400)] focus:outline-none focus:border-[var(--color-ink-600)] transition-paper"
            />
          </div>
        </div>

        <div className="mt-5 flex items-center gap-2 pt-4 border-t border-[var(--color-paper-200)]">
          <button
            onClick={handleSave}
            disabled={busy}
            className="inline-flex items-center gap-1.5 px-3.5 py-2 text-[12px] font-medium rounded-[var(--radius-sm)] bg-[var(--color-ink-800)] text-white hover:bg-[var(--color-ink-700)] disabled:opacity-50 transition-paper"
          >
            {busy ? <Loader2 size={12} className="animate-spin" /> : <Check size={12} strokeWidth={2.5} />}
            {existing ? "Update" : "Add to scope"}
          </button>
          <button
            onClick={onClose}
            className="px-3.5 py-2 text-[12px] text-[var(--color-paper-600)] hover:text-[var(--color-ink-800)] transition-paper"
          >
            Cancel
          </button>
        </div>
      </motion.div>
    </div>
  );
}

export function ArticleLibrary({ refreshKey, onChanged }: Props) {
  const [data, setData] = useState<ArticleRow[]>([]);
  const [journals, setJournals] = useState<JournalRow[]>([]);
  const [filter, setFilter] = useState("");
  const [sorting, setSorting] = useState<SortingState>([
    { id: "year", desc: true },
  ]);
  const [loading, setLoading] = useState(true);
  const [ratingArticle, setRatingArticle] = useState<ArticleRow | null>(null);

  const journalMap = useMemo(() => {
    const m = new Map<string, JournalRow>();
    for (const j of journals) m.set(normalizeJournal(j.name), j);
    return m;
  }, [journals]);

  useEffect(() => {
    setLoading(true);
    Promise.all([api.listArticles(), api.listJournals()]).then(([rows, jList]) => {
      setData(rows);
      setJournals(jList);
      setLoading(false);
    });
  }, [refreshKey]);

  const columns = useMemo<ColumnDef<ArticleRow>[]>(
    () => [
      {
        accessorKey: "id",
        header: "#",
        cell: ({ row }) => (
          <span className="font-mono text-[11px] text-[var(--color-paper-500)] tabular-nums">
            {row.original.id}
          </span>
        ),
        size: 50,
      },
      {
        accessorKey: "title",
        header: "Title",
        cell: ({ row }) => {
          const a = row.original;
          const jEntry = a.journal ? journalMap.get(normalizeJournal(a.journal)) : null;
          const hasRatings = jEntry && (jEntry.abs_rating || jEntry.abdc_rating || jEntry.impact_factor != null);
          return (
            <div className="min-w-0">
              <div className="font-serif text-[14px] text-[var(--color-ink-900)] leading-snug line-clamp-2">
                {a.title}
              </div>
              {a.authors && (
                <div className="text-[11px] text-[var(--color-paper-600)] mt-0.5 line-clamp-1">
                  {a.authors}
                </div>
              )}
              {a.journal && (
                <div className="mt-1 flex items-center gap-1.5 flex-wrap">
                  <span className="text-[11px] italic text-[var(--color-paper-500)]">
                    {a.journal}
                  </span>
                  {hasRatings && (
                    <>
                      {jEntry.abs_rating && (
                        <span className="text-[10px] font-mono font-semibold px-1 py-px rounded bg-[var(--color-paper-100)] text-[var(--color-paper-600)] border border-[var(--color-paper-300)]">
                          ABS {jEntry.abs_rating}
                        </span>
                      )}
                      {jEntry.abdc_rating && (
                        <span className="text-[10px] font-mono font-semibold px-1 py-px rounded bg-[var(--color-paper-100)] text-[var(--color-paper-600)] border border-[var(--color-paper-300)]">
                          ABDC {jEntry.abdc_rating}
                        </span>
                      )}
                      {jEntry.impact_factor != null && (
                        <span className="text-[10px] font-mono font-semibold px-1 py-px rounded bg-[var(--color-paper-100)] text-[var(--color-paper-600)] border border-[var(--color-paper-300)]">
                          IF {jEntry.impact_factor.toFixed(2)}
                        </span>
                      )}
                    </>
                  )}
                  <button
                    onClick={() => setRatingArticle(a)}
                    className={cn(
                      "text-[10px] px-1.5 py-px rounded border transition-paper",
                      hasRatings
                        ? "border-transparent text-[var(--color-paper-400)] hover:text-[var(--color-ink-600)] hover:border-[var(--color-paper-300)]"
                        : "border-[var(--color-paper-300)] text-[var(--color-paper-500)] hover:border-[var(--color-ink-400)] hover:text-[var(--color-ink-700)]"
                    )}
                  >
                    {hasRatings ? "Edit ratings" : "+ Rate"}
                  </button>
                </div>
              )}
            </div>
          );
        },
      },
      {
        accessorKey: "year",
        header: "Year",
        cell: ({ row }) => (
          <span className="font-mono text-[12px] tabular-nums text-[var(--color-paper-700)]">
            {row.original.year ?? "—"}
          </span>
        ),
        size: 70,
      },
      {
        accessorKey: "source_databases",
        header: "Source",
        cell: ({ row }) => {
          const raw = row.original.source_databases;
          if (!raw) return null;
          let names: string[] = [];
          try {
            names = JSON.parse(raw);
          } catch {
            names = [raw];
          }
          return (
            <div className="flex flex-wrap gap-1">
              {names.map((n) => (
                <span
                  key={n}
                  className="text-[10px] font-medium uppercase tracking-wider px-1.5 py-0.5 rounded-sm bg-[var(--color-paper-200)] text-[var(--color-paper-700)]"
                >
                  {n}
                </span>
              ))}
            </div>
          );
        },
        size: 160,
      },
      {
        accessorKey: "status",
        header: "Status",
        cell: ({ row }) => {
          const s = row.original.status;
          const styles =
            s === "retrieved"
              ? "bg-[var(--color-ink-100)] text-[var(--color-ink-700)]"
              : s === "included"
              ? "bg-[var(--color-ok-500)]/15 text-[var(--color-ok-500)]"
              : "bg-[var(--color-paper-200)] text-[var(--color-paper-600)]";
          return (
            <span
              className={cn(
                "text-[10px] font-medium uppercase tracking-wider px-2 py-0.5 rounded-sm",
                styles,
              )}
            >
              {s}
            </span>
          );
        },
        size: 100,
      },
      {
        accessorKey: "doi",
        header: "DOI",
        cell: ({ row }) =>
          row.original.doi ? (
            <a
              href={`https://doi.org/${row.original.doi}`}
              target="_blank"
              rel="noreferrer"
              className="font-mono text-[11px] text-[var(--color-ink-600)] hover:text-[var(--color-ink-900)] inline-flex items-center gap-1"
            >
              {row.original.doi.length > 24
                ? row.original.doi.slice(0, 24) + "…"
                : row.original.doi}
              <ExternalLink size={10} strokeWidth={1.75} />
            </a>
          ) : (
            <span className="text-[11px] text-[var(--color-paper-400)]">—</span>
          ),
        size: 200,
      },
    ],
    [journalMap],
  );

  const table = useReactTable({
    data,
    columns,
    state: { sorting, globalFilter: filter },
    onSortingChange: setSorting,
    onGlobalFilterChange: setFilter,
    globalFilterFn: "includesString",
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
  });

  if (loading) {
    return <div className="text-[13px] text-[var(--color-paper-500)]">Loading library…</div>;
  }

  if (data.length === 0) {
    return (
      <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-12 text-center max-w-2xl">
        <Library
          size={28}
          strokeWidth={1.5}
          className="mx-auto mb-3 text-[var(--color-paper-400)]"
        />
        <p className="text-sm text-[var(--color-paper-700)] mb-1 font-medium">
          The library is empty
        </p>
        <p className="text-[13px] text-[var(--color-paper-500)] max-w-md mx-auto leading-relaxed">
          Drop bibliographic exports in the <strong>Importing</strong> tab and they'll appear
          here. Each row is a unique record across all your imports.
        </p>
      </div>
    );
  }

  return (
    <div className="max-w-6xl">
      <div className="flex items-center gap-3 mb-4">
        <div className="flex-1 max-w-sm relative">
          <Search
            size={14}
            strokeWidth={1.75}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-paper-500)] pointer-events-none"
          />
          <input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter title, author, journal…"
            className="w-full h-10 pl-9 pr-3 rounded-[var(--radius-sm)] bg-[var(--color-paper-50)] border border-[var(--color-paper-300)] text-[13px] text-[var(--color-ink-800)] placeholder:text-[var(--color-paper-400)] focus:border-[var(--color-ink-600)] focus:bg-white focus:outline-none transition-paper"
          />
        </div>
        <div className="text-[12px] text-[var(--color-paper-500)] tabular-nums">
          {table.getFilteredRowModel().rows.length} of {data.length}
        </div>
      </div>

      <div className="border border-[var(--color-paper-200)] rounded-[var(--radius-md)] overflow-hidden bg-[var(--color-paper-50)]">
        <table className="w-full">
          <thead>
            {table.getHeaderGroups().map((hg) => (
              <tr key={hg.id} className="border-b border-[var(--color-paper-200)]">
                {hg.headers.map((h) => (
                  <th
                    key={h.id}
                    onClick={h.column.getToggleSortingHandler()}
                    className="text-left px-3 py-2.5 eyebrow text-[var(--color-paper-500)] cursor-default hover:text-[var(--color-ink-700)] select-none"
                    style={{ width: h.column.columnDef.size }}
                  >
                    <div className="inline-flex items-center gap-1">
                      {flexRender(h.column.columnDef.header, h.getContext())}
                      {h.column.getCanSort() && h.column.getIsSorted() && (
                        <ArrowUpDown size={10} strokeWidth={1.75} />
                      )}
                    </div>
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row, i) => (
              <tr
                key={row.id}
                className={cn(
                  "border-b last:border-b-0 border-[var(--color-paper-200)] transition-paper",
                  i % 2 === 0 ? "bg-[var(--color-paper-50)]" : "bg-white",
                  "hover:bg-[var(--color-paper-100)]",
                )}
              >
                {row.getVisibleCells().map((cell) => (
                  <td key={cell.id} className="px-3 py-3 align-top">
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* ── Quick-rate modal ───────────────────────────────────────────── */}
      <AnimatePresence>
        {ratingArticle && ratingArticle.journal && (
          <QuickRateModal
            journalName={ratingArticle.journal}
            existing={journalMap.get(normalizeJournal(ratingArticle.journal)) ?? null}
            onSave={(saved) => {
              setJournals((prev) => {
                const idx = prev.findIndex((j) => j.id === saved.id);
                if (idx >= 0) {
                  const next = [...prev];
                  next[idx] = saved;
                  return next;
                }
                return [...prev, saved];
              });
              setRatingArticle(null);
              onChanged();
            }}
            onClose={() => setRatingArticle(null)}
          />
        )}
      </AnimatePresence>
    </div>
  );
}
