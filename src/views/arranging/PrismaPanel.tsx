import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { ArrowRight, Download, FileImage, GitBranch, Loader2 } from "lucide-react";
import { api, type PrismaState } from "../../lib/api";
import { downloadBlob, svgToPngBlob } from "../../lib/export";
import { Card } from "../../components/ui/Card";

interface Props {
  refreshKey: number;
}

export function PrismaPanel({ refreshKey }: Props) {
  const [state, setState] = useState<PrismaState | null>(null);
  const [svg, setSvg] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [exporting, setExporting] = useState<"svg" | "png" | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    Promise.all([api.getPrismaState(), api.getPrismaSvg().catch(() => null)]).then(([s, x]) => {
      setState(s);
      setSvg(x);
      setLoading(false);
    });
  }, [refreshKey]);

  const stamp = () => new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);

  const handleExportSvg = async () => {
    setExporting("svg");
    setExportError(null);
    try {
      const text = svg ?? (await api.getPrismaSvg());
      downloadBlob(`prisma-flow-${stamp()}.svg`, new Blob([text], { type: "image/svg+xml;charset=utf-8" }));
    } catch (e: unknown) {
      setExportError(e instanceof Error ? e.message : String(e));
    } finally {
      setExporting(null);
    }
  };

  const handleExportPng = async () => {
    setExporting("png");
    setExportError(null);
    try {
      const text = svg ?? (await api.getPrismaSvg());
      const png = await svgToPngBlob(text, 2);
      downloadBlob(`prisma-flow-${stamp()}.png`, png);
    } catch (e: unknown) {
      setExportError(e instanceof Error ? e.message : String(e));
    } finally {
      setExporting(null);
    }
  };

  if (loading || !state) {
    return (
      <div className="flex items-center gap-2 text-[13px] text-[var(--color-paper-500)]">
        <Loader2 size={14} className="animate-spin" /> Computing PRISMA flow…
      </div>
    );
  }

  const sought = state.identified_databases - state.duplicates_removed;

  return (
    <div className="pl-6 pr-4 xl:pl-12 xl:pr-8">
      <div className="mb-8 flex items-start justify-between gap-6">
        <div>
          <div className="eyebrow text-[var(--color-paper-500)] flex items-center gap-2">
            <GitBranch size={11} strokeWidth={1.75} />
            PRISMA 2020 flow diagram
          </div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">
            Live record flow
          </h2>
          <p className="text-[13px] text-[var(--color-paper-600)] mt-2 max-w-2xl leading-relaxed">
            Counts update automatically from your project database. This is the diagram you'll
            include in the methods section of your paper, with the same numbers a journal reviewer
            would compute themselves from your CSVs.
          </p>
        </div>
        <div className="shrink-0 flex items-center gap-2">
          <button
            type="button"
            onClick={handleExportSvg}
            disabled={exporting !== null}
            className="inline-flex items-center gap-1.5 px-3 py-2 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-accent-700)] border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)] transition-paper disabled:opacity-50 disabled:cursor-wait"
            title="Vector SVG — sharpest for papers, scales without pixelation"
          >
            {exporting === "svg" ? <Loader2 size={13} className="animate-spin" /> : <Download size={13} strokeWidth={2} />}
            {exporting === "svg" ? "Exporting…" : "Export SVG"}
          </button>
          <button
            type="button"
            onClick={handleExportPng}
            disabled={exporting !== null}
            className="inline-flex items-center gap-1.5 px-3 py-2 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-paper-600)] border border-[var(--color-paper-300)] bg-white hover:bg-[var(--color-paper-100)] transition-paper disabled:opacity-50 disabled:cursor-wait"
            title="PNG image rasterised from the SVG at 2× DPI"
          >
            {exporting === "png" ? <Loader2 size={13} className="animate-spin" /> : <FileImage size={13} strokeWidth={2} />}
            {exporting === "png" ? "Exporting…" : "Export PNG"}
          </button>
        </div>
      </div>
      {exportError && (
        <p className="mb-3 text-[12px] text-[var(--color-error-600)]">{exportError}</p>
      )}

      <div>
      <Card className="paper-grain p-10">
        {/* PRISMA layout — three vertical lanes */}
        <div className="space-y-10">
          {/* IDENTIFICATION */}
          <Lane label="Identification">
            <div className="grid grid-cols-[1fr_auto_1fr] gap-6 items-center">
              <PrismaBox
                title="Records identified from databases"
                count={state.identified_databases}
                tone="primary"
              />
              <Arrow />
              <PrismaBox
                title="Records removed before screening"
                items={[
                  { label: "Duplicates removed", count: state.duplicates_removed },
                  { label: "Marked ineligible by automation", count: 0 },
                  { label: "Other reasons", count: 0 },
                ]}
                tone="muted"
              />
            </div>
          </Lane>

          <DownArrow />

          {/* SCREENING */}
          <Lane label="Screening">
            <div className="grid grid-cols-[1fr_auto_1fr] gap-6 items-center">
              <PrismaBox
                title="Records screened"
                count={sought}
                tone="primary"
              />
              <Arrow />
              <PrismaBox
                title="Records excluded"
                count={state.records_excluded}
                items={state.by_criterion
                  .filter(([, n]) => n > 0)
                  .map(([label, n]) => ({ label, count: n }))}
                tone="muted"
              />
            </div>
          </Lane>

          <DownArrow />

          {/* INCLUDED */}
          <Lane label="Included">
            <div className="flex justify-center">
              <div className="w-[60%]">
                <PrismaBox
                  title="Studies included in review"
                  count={state.records_included}
                  tone="ink"
                  large
                />
              </div>
            </div>
          </Lane>
        </div>
      </Card>
      </div>

      {svg && (
        <div className="mt-8">
          <div className="eyebrow text-[var(--color-paper-500)] mb-2">Exported diagram preview</div>
          <p className="text-[12px] text-[var(--color-paper-600)] mb-3 max-w-2xl leading-relaxed">
            This is exactly what gets saved when you click Export — clean vector geometry, no UI chrome.
            Pixel-identical at any zoom level and copy-paste-able into your manuscript.
          </p>
          <div
            className="rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] p-4 overflow-auto"
            // The Rust renderer escapes user-supplied strings (project name,
            // criterion labels) before assembling the SVG, so injecting the
            // markup here is safe.
            dangerouslySetInnerHTML={{ __html: svg }}
          />
        </div>
      )}

      <p className="mt-4 text-[11px] text-[var(--color-paper-500)] italic">
        Compatible with the PRISMA 2020 flow template (Page et al., 2021).
      </p>
    </div>
  );
}

function Lane({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[120px_1fr] gap-6 items-start">
      <div className="pt-2 text-right">
        <div className="eyebrow text-[var(--color-paper-500)]">{label}</div>
      </div>
      <div>{children}</div>
    </div>
  );
}

function PrismaBox({
  title,
  count,
  items,
  tone,
  large,
}: {
  title: string;
  count?: number;
  items?: { label: string; count: number }[];
  tone: "primary" | "muted" | "ink";
  large?: boolean;
}) {
  const toneClass =
    tone === "ink"
      ? "bg-[var(--color-ink-800)] text-[var(--color-paper-50)] border-[var(--color-ink-800)]"
      : tone === "primary"
      ? "bg-white border-[var(--color-paper-400)]"
      : "bg-[var(--color-paper-100)] border-[var(--color-paper-300)]";
  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.96 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.4, ease: [0.2, 0.7, 0.2, 1] }}
      className={`p-4 rounded-[var(--radius-md)] border-2 ${toneClass}`}
    >
      <div
        className={`text-[12px] font-medium leading-snug ${
          tone === "ink" ? "text-[var(--color-paper-200)]" : "text-[var(--color-ink-800)]"
        }`}
      >
        {title}
      </div>
      {count !== undefined && (
        <div
          className={`font-serif tabular-nums leading-none mt-2 ${
            large ? "text-[44px]" : "text-[28px]"
          } ${tone === "ink" ? "text-[var(--color-paper-50)]" : "text-[var(--color-ink-900)]"}`}
        >
          n = {count}
        </div>
      )}
      {items && items.length > 0 && (
        <ul className="mt-2 space-y-0.5">
          {items.map((it, i) => (
            <li
              key={i}
              className={`text-[11px] flex justify-between gap-3 ${
                tone === "ink"
                  ? "text-[var(--color-paper-200)]"
                  : "text-[var(--color-paper-600)]"
              }`}
            >
              <span>{it.label}</span>
              <span className="font-mono tabular-nums">n = {it.count}</span>
            </li>
          ))}
        </ul>
      )}
    </motion.div>
  );
}

function Arrow() {
  return (
    <ArrowRight
      size={20}
      strokeWidth={1.5}
      className="text-[var(--color-paper-400)] mx-auto"
    />
  );
}

function DownArrow() {
  return (
    <div className="grid grid-cols-[120px_1fr] gap-6">
      <div />
      <div className="flex justify-start ml-[12%]">
        <div className="w-px h-6 bg-[var(--color-paper-300)] relative">
          <div className="absolute -left-[3px] -bottom-[1px] w-[7px] h-[7px] border-r border-b border-[var(--color-paper-400)] transform rotate-45" />
        </div>
      </div>
    </div>
  );
}
