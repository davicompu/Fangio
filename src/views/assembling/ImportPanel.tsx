import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { listen } from "@tauri-apps/api/event";
import { motion, AnimatePresence } from "framer-motion";
import {
  AlertTriangle,
  CheckCircle2,
  ExternalLink,
  FileText,
  Globe,
  Loader2,
  Sparkles,
  Upload,
  X,
} from "lucide-react";
import { api, type ParsePreview } from "../../lib/api";
import { Button } from "../../components/ui/Button";
import { Card } from "../../components/ui/Card";
import { Select } from "../../components/ui/Select";

interface Props {
  onCommitted: () => void;
  /** Labels of the databases the researcher selected on the Acquisition tab.
   *  When non-empty, the Source database dropdown is restricted to this list. */
  availableDatabases?: string[];
  /** Selected databases that have a search URL — rendered as quick-launch buttons,
   *  mirroring the "Run searches" row on the Acquisition tab. */
  launchDatabases?: { label: string; url: string }[];
}

export function ImportPanel({ onCommitted, availableDatabases = [], launchDatabases = [] }: Props) {
  const handleOpenDatabase = async (url: string) => {
    let target = url;
    if (!/^https?:\/\//i.test(target)) target = "https://" + target;
    try {
      await openUrl(target);
    } catch (e) {
      console.error("Failed to open URL:", e);
    }
  };
  const [preview, setPreview] = useState<ParsePreview | null>(null);
  const [filePath, setFilePath] = useState<string | null>(null);
  const [database, setDatabase] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [committed, setCommitted] = useState<{
    inserted: number;
    skipped: number;
  } | null>(null);
  const [isDragOver, setIsDragOver] = useState(false);

  useEffect(() => {
    const SUPPORTED = [".ris", ".bib", ".bibtex", ".txt", ".tsv", ".csv"];
    let unlistenDrop: (() => void) | null = null;
    let unlistenEnter: (() => void) | null = null;
    let unlistenLeave: (() => void) | null = null;

    listen<{ paths: string[] }>("tauri://drag-drop", async (event) => {
      setIsDragOver(false);
      const dropped = event.payload.paths.find((p) =>
        SUPPORTED.some((ext) => p.toLowerCase().endsWith(ext))
      );
      if (!dropped) {
        setError("Unsupported file type. Drop a .ris, .bib, .txt (WoS), or .csv (Scopus) file.");
        return;
      }
      setError(null);
      setCommitted(null);
      setFilePath(dropped);
      setBusy(true);
      try {
        const p = await api.parsePreview(dropped);
        setPreview(p);
        if (!database) {
          const guess = guessDatabase(p.format, availableDatabases);
          if (guess) setDatabase(guess);
        }
      } catch (e: any) {
        setError(e?.message || String(e));
      } finally {
        setBusy(false);
      }
    }).then((fn) => { unlistenDrop = fn; });

    listen("tauri://drag-enter", () => setIsDragOver(true)).then((fn) => { unlistenEnter = fn; });
    listen("tauri://drag-leave", () => setIsDragOver(false)).then((fn) => { unlistenLeave = fn; });

    return () => {
      unlistenDrop?.();
      unlistenEnter?.();
      unlistenLeave?.();
    };
  }, []);

  const pickFile = async () => {
    setError(null);
    setCommitted(null);
    const selected = await open({
      multiple: false,
      title: "Select an export file",
      filters: [
        {
          name: "Bibliographic exports",
          extensions: ["ris", "bib", "bibtex", "txt", "tsv", "csv"],
        },
      ],
    });
    if (!selected || Array.isArray(selected)) return;
    const path = selected as string;
    setFilePath(path);
    setBusy(true);
    try {
      const p = await api.parsePreview(path);
      setPreview(p);
      // Suggest a default database label from the format, but only when the
      // guess is one the researcher actually disclosed.
      if (!database) {
        const guess = guessDatabase(p.format, availableDatabases);
        if (guess) setDatabase(guess);
      }
    } catch (e: any) {
      setError(e?.message || String(e));
    } finally {
      setBusy(false);
    }
  };

  const commit = async () => {
    if (!filePath || !database.trim()) {
      setError("Pick a file and confirm the source database name.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const r = await api.commitImport(filePath, database.trim());
      setCommitted({ inserted: r.inserted, skipped: r.skipped_doi_dupes });
      setPreview(null);
      setFilePath(null);
      onCommitted();
    } catch (e: any) {
      setError(e?.message || String(e));
    } finally {
      setBusy(false);
    }
  };

  const reset = () => {
    setPreview(null);
    setFilePath(null);
    setError(null);
  };

  return (
    <div className="grid grid-cols-[1fr_320px] gap-10 max-w-6xl">
      <div>
        {!preview && (
          <Dropzone onPick={pickFile} busy={busy} isDragOver={isDragOver} />
        )}

        <AnimatePresence>
          {preview && (
            <motion.div
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
            >
              <Card className="p-6 mb-6">
                <div className="flex items-start justify-between mb-4">
                  <div>
                    <div className="eyebrow text-[var(--color-paper-500)]">Parsed</div>
                    <h3 className="font-serif text-[22px] tracking-tight text-[var(--color-ink-900)] mt-1">
                      {preview.records.length} records
                    </h3>
                    <p className="text-[13px] text-[var(--color-paper-600)] mt-1">
                      {preview.format_label} · <span className="font-mono">{preview.filename}</span>
                    </p>
                  </div>
                  <button
                    onClick={reset}
                    className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
                  >
                    <X size={15} strokeWidth={1.75} />
                  </button>
                </div>

                {preview.warnings.length > 0 && (
                  <div className="mb-4 px-3.5 py-2.5 rounded-[var(--radius-sm)] bg-[var(--color-warn-500)]/10 border border-[var(--color-warn-500)]/30 text-[12px] text-[var(--color-warn-500)] flex gap-2">
                    <AlertTriangle size={13} strokeWidth={1.75} className="shrink-0 mt-0.5" />
                    <div>
                      <strong>{preview.warnings.length} warning{preview.warnings.length === 1 ? "" : "s"}</strong>
                      <ul className="mt-1 space-y-0.5 max-h-20 overflow-y-auto">
                        {preview.warnings.slice(0, 5).map((w, i) => (
                          <li key={i} className="font-mono text-[11px]">• {w}</li>
                        ))}
                      </ul>
                    </div>
                  </div>
                )}

                <div className="mb-5">
                  {availableDatabases.length === 0 ? (
                    <div className="px-3.5 py-3 rounded-[var(--radius-sm)] bg-[var(--color-accent-50)] border border-[var(--color-accent-300)] text-[12px] text-[var(--color-accent-700)] leading-relaxed">
                      <strong>No databases selected.</strong> Open the Acquisition tab and tick the
                      databases you'll search before importing — Reeves only lets you label imports
                      with sources you've disclosed in your method.
                    </div>
                  ) : (
                    <Select
                      label="Source database"
                      value={database}
                      onChange={(e) => setDatabase(e.target.value)}
                      hint="Restricted to the databases you selected on the Acquisition tab."
                    >
                      <option value="">— pick the source —</option>
                      {availableDatabases.map((label) => (
                        <option key={label} value={label}>{label}</option>
                      ))}
                    </Select>
                  )}
                </div>

                {/* Preview table — first 5 records */}
                <div className="border border-[var(--color-paper-200)] rounded-[var(--radius-sm)] overflow-hidden">
                  {preview.records.slice(0, 5).map((r, i) => (
                    <div
                      key={i}
                      className="px-4 py-3 border-b last:border-b-0 border-[var(--color-paper-200)] bg-[var(--color-paper-50)]"
                    >
                      <div className="flex items-baseline gap-3">
                        <span className="font-mono text-[10px] text-[var(--color-paper-500)] tabular-nums">
                          {String(i + 1).padStart(2, "0")}
                        </span>
                        <h4 className="font-serif text-[14px] leading-snug text-[var(--color-ink-900)] flex-1 line-clamp-2">
                          {r.title}
                        </h4>
                        {r.year && (
                          <span className="font-mono text-[11px] text-[var(--color-paper-500)]">
                            {r.year}
                          </span>
                        )}
                      </div>
                      {r.authors && (
                        <p className="ml-7 mt-1 text-[12px] text-[var(--color-paper-600)] line-clamp-1">
                          {r.authors}
                        </p>
                      )}
                      {r.journal && (
                        <p className="ml-7 mt-0.5 text-[11px] italic text-[var(--color-paper-500)] line-clamp-1">
                          {r.journal}
                        </p>
                      )}
                    </div>
                  ))}
                  {preview.records.length > 5 && (
                    <div className="px-4 py-2.5 bg-[var(--color-paper-100)] text-[11px] text-[var(--color-paper-500)] text-center">
                      … and {preview.records.length - 5} more
                    </div>
                  )}
                </div>

                <div className="mt-5 flex items-center gap-3">
                  <Button
                    onClick={commit}
                    disabled={busy || !database.trim()}
                    variant="primary"
                  >
                    {busy ? (
                      <>
                        <Loader2 size={14} className="animate-spin" /> Committing
                      </>
                    ) : (
                      <>
                        Add {preview.records.length} records to library
                        <Sparkles size={13} strokeWidth={2} />
                      </>
                    )}
                  </Button>
                  <Button onClick={reset} variant="ghost">
                    Cancel
                  </Button>
                </div>
              </Card>
            </motion.div>
          )}
        </AnimatePresence>

        <AnimatePresence>
          {committed && (
            <motion.div
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
              className="px-4 py-3 rounded-[var(--radius-sm)] bg-[var(--color-ok-500)]/10 border border-[var(--color-ok-500)]/30 flex items-center gap-2.5"
            >
              <CheckCircle2
                size={16}
                strokeWidth={1.75}
                className="text-[var(--color-ok-500)]"
              />
              <span className="text-[13px] text-[var(--color-ink-800)]">
                Added <strong>{committed.inserted}</strong> records to the library
              </span>
            </motion.div>
          )}
        </AnimatePresence>

        {error && (
          <div className="mt-4 px-4 py-3 rounded-[var(--radius-sm)] bg-[var(--color-accent-50)] border border-[var(--color-accent-300)] text-[13px] text-[var(--color-accent-700)]">
            {error}
          </div>
        )}
      </div>

      <aside>
        <div className="sticky top-12 space-y-3">
          {launchDatabases.length > 0 && (
            <Card className="p-5">
              <div className="flex items-center gap-2 mb-3">
                <Globe size={13} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                <span className="eyebrow text-[var(--color-paper-500)]">Run another search</span>
              </div>
              <p className="text-[12px] text-[var(--color-paper-600)] leading-relaxed mb-3">
                Reopen a database to gather more results, then export and drop them here.
              </p>
              <div className="flex flex-wrap gap-2">
                {launchDatabases.map((d) => (
                  <button
                    key={d.label}
                    type="button"
                    onClick={() => handleOpenDatabase(d.url)}
                    className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium bg-[var(--color-paper-100)] border border-[var(--color-paper-300)] text-[var(--color-ink-800)] hover:border-[var(--color-paper-500)] transition-paper"
                  >
                    {d.label}
                    <ExternalLink size={11} strokeWidth={1.75} className="text-[var(--color-paper-400)]" />
                  </button>
                ))}
              </div>
            </Card>
          )}
          <Card className="p-5">
            <div className="eyebrow mb-3 text-[var(--color-paper-500)]">Supported formats</div>
            <ul className="space-y-2.5 text-[13px] text-[var(--color-paper-700)]">
              <FormatRow name="RIS" tag=".ris" desc="Universal — every database exports this" />
              <FormatRow name="BibTeX" tag=".bib" desc="Common in CS / engineering" />
              <FormatRow name="WoS tab-delimited" tag=".txt" desc="Web of Science 'Full Record'" />
              <FormatRow name="Scopus CSV" tag=".csv" desc="Native Scopus export" />
            </ul>
            <div className="hairline my-4" />
            <p className="text-[12px] text-[var(--color-paper-600)] leading-relaxed">
              Reeves auto-detects the format from extension and content. All records are imported
              — cross-database duplicates surface in the Deduplication tab for your review.
            </p>
          </Card>
        </div>
      </aside>
    </div>
  );
}

function Dropzone({ onPick, busy, isDragOver }: { onPick: () => void; busy: boolean; isDragOver: boolean }) {
  return (
    <button
      type="button"
      onClick={onPick}
      disabled={busy}
      className={
        "w-full p-12 border-2 border-dashed rounded-[var(--radius-lg)] transition-paper text-center group disabled:opacity-50 " +
        (isDragOver
          ? "border-[var(--color-ink-700)] bg-[var(--color-paper-100)] ring-2 ring-[var(--color-ink-700)]/20"
          : "border-[var(--color-paper-300)] bg-[var(--color-paper-50)] hover:bg-white hover:border-[var(--color-paper-500)]")
      }
    >
      <div className="w-14 h-14 rounded-full bg-[var(--color-paper-200)] mx-auto mb-4 flex items-center justify-center group-hover:bg-[var(--color-paper-300)] transition-paper">
        {busy ? (
          <Loader2 size={22} className="animate-spin text-[var(--color-ink-700)]" />
        ) : (
          <Upload size={22} strokeWidth={1.75} className="text-[var(--color-ink-700)]" />
        )}
      </div>
      <h3 className="font-serif text-[20px] tracking-tight text-[var(--color-ink-900)] mb-1">
        {busy ? "Parsing…" : isDragOver ? "Release to import" : "Drop a bibliographic export"}
      </h3>
      <p className="text-[13px] text-[var(--color-paper-600)] max-w-md mx-auto leading-relaxed">
        Drag a file here or click to choose. Reeves handles RIS, BibTeX, Web of Science
        tab-delimited, and Scopus CSV.
      </p>
    </button>
  );
}

/** Guess a database label from the parsed format, but only return labels
 *  that appear in the researcher's disclosed list. */
function guessDatabase(format: string, available: string[]): string | null {
  const findCaseInsensitive = (needle: string) =>
    available.find((d) => d.toLowerCase().includes(needle.toLowerCase())) ?? null;
  if (format === "scopus-csv") return findCaseInsensitive("scopus");
  if (format === "wos-tab") return findCaseInsensitive("web of science") ?? findCaseInsensitive("wos");
  return null;
}

function FormatRow({ name, tag, desc }: { name: string; tag: string; desc: string }) {
  return (
    <li className="flex items-start gap-2">
      <FileText
        size={13}
        strokeWidth={1.75}
        className="text-[var(--color-paper-500)] mt-0.5 shrink-0"
      />
      <div className="min-w-0">
        <span className="font-medium text-[var(--color-ink-800)]">{name}</span>
        <span className="font-mono text-[11px] text-[var(--color-paper-500)] ml-1.5">{tag}</span>
        <p className="text-[12px] text-[var(--color-paper-600)] leading-relaxed">{desc}</p>
      </div>
    </li>
  );
}
