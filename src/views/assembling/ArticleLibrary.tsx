import { useEffect, useMemo, useState } from "react";
import {
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
  type ColumnDef,
  type SortingState,
} from "@tanstack/react-table";
import { ArrowUpDown, ExternalLink, Library, Search } from "lucide-react";
import { api, type ArticleRow } from "../../lib/api";
import { cn } from "../../lib/utils";

interface Props {
  refreshKey: number;
  onChanged: () => void;
}

export function ArticleLibrary({ refreshKey }: Props) {
  const [data, setData] = useState<ArticleRow[]>([]);
  const [filter, setFilter] = useState("");
  const [sorting, setSorting] = useState<SortingState>([
    { id: "year", desc: true },
  ]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    api.listArticles().then((rows) => {
      setData(rows);
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
        cell: ({ row }) => (
          <div className="min-w-0">
            <div className="font-serif text-[14px] text-[var(--color-ink-900)] leading-snug line-clamp-2">
              {row.original.title}
            </div>
            {row.original.authors && (
              <div className="text-[11px] text-[var(--color-paper-600)] mt-0.5 line-clamp-1">
                {row.original.authors}
              </div>
            )}
            {row.original.journal && (
              <div className="text-[11px] italic text-[var(--color-paper-500)] line-clamp-1">
                {row.original.journal}
              </div>
            )}
          </div>
        ),
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
    [],
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
    </div>
  );
}
