import { useEffect, useRef } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { AlertTriangle, Bot, X } from "lucide-react";
import { api, type AiScreeningLogEntry, type ReviewStatus } from "../lib/api";
import { cn } from "../lib/utils";

const STATUS_LABEL: Record<ReviewStatus, string> = {
  auto_applied: "Applied automatically",
  opened: "Opened, left unchanged",
  overridden: "Overridden",
  pending: "Pending",
};

const STATUS_CLASS: Record<ReviewStatus, string> = {
  auto_applied:
    "bg-[var(--color-accent-50)] border-[var(--color-accent-300)] text-[var(--color-accent-700)]",
  opened:
    "bg-[var(--color-paper-100)] border-[var(--color-paper-300)] text-[var(--color-paper-600)]",
  overridden:
    "bg-[var(--color-warn-50)] border-[var(--color-warn-300)] text-[var(--color-warn-700)]",
  pending:
    "bg-transparent border-[var(--color-warn-500)] text-[var(--color-warn-700)]",
};

export function ReviewStatusBadge({ status }: { status: ReviewStatus }) {
  return (
    <span
      className={cn(
        "inline-flex items-center px-1.5 py-px rounded-full border text-[10px] font-medium whitespace-nowrap not-italic",
        STATUS_CLASS[status],
      )}
    >
      {STATUS_LABEL[status]}
    </span>
  );
}

/** Muted text for a real AI decision whose review status can't be derived
 *  (the article was decided outside the AI log, e.g. before it existed). */
export const UNDETERMINED = "Can't be determined";

/** True for include/exclude/uncertain rows; false for error/skipped rows. */
export function hasRealDecision(e: AiScreeningLogEntry): boolean {
  return e.decision !== "error" && e.decision !== "skipped";
}

/** Pick the latest AI log entry per article from a batch of log rows. */
export function latestByArticle(entries: AiScreeningLogEntry[]): Map<number, AiScreeningLogEntry> {
  const m = new Map<number, AiScreeningLogEntry>();
  for (const e of entries) if (e.is_latest) m.set(e.article_id, e);
  return m;
}

/** Card line summarising the latest logged AI decision, with its review badge.
 *  Clicking opens the detail dialog (the caller owns the dialog state). */
export function AiDecisionLine({
  entry,
  onOpen,
}: {
  entry: AiScreeningLogEntry;
  onOpen: () => void;
}) {
  const noDecision = !hasRealDecision(entry);
  const muted = noDecision
    ? entry.decision === "skipped"
      ? "Skipped — no PDF"
      : "AI error — no decision"
    : null;
  return (
    <button
      onClick={onOpen}
      className="mt-2 flex items-start gap-1.5 text-[11px] text-left w-full group"
    >
      <Bot size={11} strokeWidth={1.75} className="shrink-0 mt-0.5 text-[var(--color-accent-500)]" />
      <span className="flex-1 min-w-0 leading-snug text-[var(--color-paper-500)] group-hover:underline">
        {muted ? (
          <span className="italic text-[var(--color-paper-400)]">{muted}</span>
        ) : (
          <>
            {entry.decision === "uncertain" && (
              <span className="font-medium text-[var(--color-warn-600)] mr-1">AI couldn't decide —</span>
            )}
            <span className="line-clamp-2">{cleanScreenerRationale(entry.rationale)}</span>
            {entry.confidence !== null && (
              <span className="ml-1 text-[var(--color-paper-400)]">
                ({Math.round(entry.confidence * 100)}% confidence)
              </span>
            )}
          </>
        )}
      </span>
      {entry.review_status ? (
        <span className="shrink-0">
          <ReviewStatusBadge status={entry.review_status} />
        </span>
      ) : (
        !noDecision && (
          <span className="shrink-0 text-[10px] italic text-[var(--color-paper-400)] whitespace-nowrap">
            {UNDETERMINED}
          </span>
        )
      )}
    </button>
  );
}

function fmtTime(raw: string | null): string {
  if (!raw) return "—";
  // SQLite stores UTC as "YYYY-MM-DD HH:MM:SS"; treat zone-less values as UTC.
  let s = raw.trim().replace(" ", "T");
  if (!/(Z|[+-]\d{2}:?\d{2})$/.test(s)) s += "Z";
  const d = new Date(s);
  if (Number.isNaN(d.getTime())) return raw;
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

function fmtTemp(t: number | null): string {
  if (t === null) return "—";
  return String(Math.round(t * 100) / 100);
}

function decisionLabel(e: AiScreeningLogEntry): string {
  switch (e.decision) {
    case "include":
      return "Include";
    case "exclude":
      return e.criterion_number != null ? `Exclude (EC${e.criterion_number})` : "Exclude";
    case "uncertain":
      return "Uncertain";
    case "skipped":
      return "Skipped";
    default:
      return "Error";
  }
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex gap-3 text-[12px]">
      <dt className="w-28 shrink-0 text-[var(--color-paper-500)]">{label}</dt>
      <dd className="flex-1 min-w-0 text-[var(--color-ink-800)] break-words">{children}</dd>
    </div>
  );
}

/** Logged AI decision detail. Mounting records one open (unless the entry has
 *  no decision, i.e. review_status is null). The opens summary renders from the
 *  entry as loaded, so "Never opened" stays visible on the first open. */
export function AiDecisionDetailDialog({
  entry,
  onClose,
}: {
  entry: AiScreeningLogEntry;
  onClose: () => void;
}) {
  const recordedFor = useRef<number | null>(null);
  useEffect(() => {
    if (entry.review_status === null) return;
    if (recordedFor.current === entry.id) return;
    recordedFor.current = entry.id;
    api.recordAiScreeningOpen(entry.id).catch(() => {});
  }, [entry.id, entry.review_status]);

  const hasDecision = hasRealDecision(entry);
  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        transition={{ duration: 0.12 }}
        className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/30 backdrop-blur-sm"
        onClick={onClose}
      >
        <motion.div
          initial={{ opacity: 0, scale: 0.97, y: 6 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.97, y: 6 }}
          transition={{ duration: 0.15 }}
          className="w-full max-w-md max-h-[85vh] flex flex-col bg-white rounded-[var(--radius-lg)] border border-[var(--color-paper-300)] shadow-[var(--shadow-paper-lg)] overflow-hidden"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="flex items-center gap-2 px-5 py-3.5 border-b border-[var(--color-paper-200)]">
            <Bot size={14} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
            <span className="text-[13px] font-semibold text-[var(--color-ink-900)] flex-1">
              AI screener — logged decision
            </span>
            {entry.review_status ? (
              <ReviewStatusBadge status={entry.review_status} />
            ) : (
              hasDecision && (
                <span className="text-[10px] italic text-[var(--color-paper-400)]">{UNDETERMINED}</span>
              )
            )}
            <button
              onClick={onClose}
              className="p-1 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] text-[var(--color-paper-500)] transition-paper"
            >
              <X size={13} strokeWidth={2} />
            </button>
          </div>
          <div className="px-5 py-4 space-y-4 overflow-y-auto">
            <dl className="space-y-1.5">
              <Row label="Decision">{decisionLabel(entry)}</Row>
              <Row label="Confidence">
                {entry.confidence !== null ? `${Math.round(entry.confidence * 100)}%` : "—"}
              </Row>
              <Row label="Model used">{entry.model_used ?? entry.model_name ?? "—"}</Row>
              <Row label="Prompt fingerprint">
                <span className="font-mono text-[11px]">{entry.prompt_hash ?? "—"}</span>
              </Row>
              <Row label="Temperature">{fmtTemp(entry.temperature)}</Row>
              {entry.auto_applied && <Row label="Applied">Applied automatically</Row>}
              {hasDecision && (
                <Row label="Opens">
                  {entry.open_count === 0 ? (
                    <span className="font-medium">Never opened</span>
                  ) : (
                    `Opened ${entry.open_count} ${entry.open_count === 1 ? "time" : "times"} · first ${fmtTime(
                      entry.first_opened_at,
                    )} · last ${fmtTime(entry.last_opened_at)}${
                      entry.last_opened_by ? ` · last opened by ${entry.last_opened_by}` : ""
                    }`
                  )}
                </Row>
              )}
              {hasDecision && entry.review_status === null && (
                <Row label="Review status">
                  <span className="italic text-[var(--color-paper-500)]">
                    {UNDETERMINED} — decided outside the AI log
                  </span>
                </Row>
              )}
            </dl>
            {hasDecision ? (
              <RationaleBody text={entry.rationale} confidence={null} />
            ) : (
              <p className="text-[12px] italic text-[var(--color-paper-500)]">
                {entry.decision === "skipped" ? "Skipped — no PDF" : "AI error — no decision"}
                {entry.rationale ? `: ${entry.rationale}` : ""}
              </p>
            )}
          </div>
          <div className="px-5 py-3 border-t border-[var(--color-paper-200)] text-[11px] text-[var(--color-paper-500)] leading-snug">
            Opening is logged. Reeves records that this decision was opened, not that it was assessed.
          </div>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  );
}

/** The AI screener is a single rater. Legacy rows may have stored a combined
 *  "Rater 1: …\n\nRater 2: …" string (from the old dual-rater screener); since
 *  that was one model's output duplicated, collapse to the first block and drop
 *  the rater labels. New rows have no "Rater N:" prefix and pass through. */
export function cleanScreenerRationale(text: string): string {
  const t = (text ?? "").trim();
  if (!t) return "";
  const matches = [...t.matchAll(/(^|\n+)\s*Rater\s+\d+:\s*/g)];
  if (matches.length === 0) return t;
  const first = matches[0];
  const firstStart = (first.index ?? 0) + first[0].length;
  const secondStart = matches[1]?.index ?? t.length;
  return t.slice(firstStart, secondStart).trim();
}

export function RationaleBody({ text, confidence }: { text: string; confidence: number | null }) {
  const rawBody = cleanScreenerRationale(text);
  const isWeak = rawBody.startsWith("[weak rationale");
  const cleanBody = isWeak ? rawBody.replace(/^\[weak rationale[^\]]*\]\s*/, "") : rawBody;
  const sentences = splitSentences(cleanBody);
  return (
    <div className="space-y-4">
      <div className="relative pl-3 border-l-2 border-[var(--color-accent-300)]">
        {isWeak && (
          <div className="flex items-start gap-1.5 mb-1.5 text-[11px] text-[var(--color-warn-700)]">
            <AlertTriangle size={11} strokeWidth={2} className="shrink-0 mt-0.5" />
            <span>Weak rationale — recommend manual review.</span>
          </div>
        )}
        <ul className="text-[13px] text-[var(--color-ink-800)] leading-relaxed divide-y divide-[var(--color-paper-200)]">
          {sentences.map((s, j) => (
            <li key={j} className="flex gap-2 py-1.5 first:pt-0 last:pb-0">
              <span className="font-mono text-[10px] text-[var(--color-paper-400)] shrink-0 select-none mt-1 tabular-nums">
                {j + 1}
              </span>
              <span className="flex-1">
                {extractEvidence(s).map((frag, k) =>
                  frag.evidence ? (
                    <span
                      key={k}
                      className="font-medium text-[var(--color-ink-900)] bg-[var(--color-accent-50)] border-b border-[var(--color-accent-300)] px-0.5 rounded-sm"
                    >
                      "{frag.text}"
                    </span>
                  ) : (
                    <span key={k}>{frag.text}</span>
                  ),
                )}
              </span>
            </li>
          ))}
        </ul>
      </div>
      {confidence !== null && (
        <div className="text-[11px] text-[var(--color-paper-500)] font-mono pt-2 border-t border-[var(--color-paper-200)]">
          {Math.round(confidence * 100)}% confidence
        </div>
      )}
    </div>
  );
}

/** Split a rationale into 1-N sentences for cleaner reading. Conservative —
 *  splits on `. `/`! `/`? ` followed by an uppercase letter or end of string,
 *  preserves trailing punctuation, drops empties. Single-sentence rationales
 *  come back as a one-element array. */
export function splitSentences(text: string): string[] {
  const trimmed = text.trim();
  if (!trimmed) return [];
  // Look-behind for sentence-ending punctuation, look-ahead for whitespace then capital or end.
  const parts = trimmed
    .split(/(?<=[.!?])\s+(?=[A-Z“"'(\[]|$)/u)
    .map((s) => s.trim())
    .filter(Boolean);
  return parts.length > 0 ? parts : [trimmed];
}

/** Pull out anything in straight or curly quotes so the UI can emphasise it as
 *  evidence. Returns the rationale text broken into ordered fragments
 *  ({text} | {evidence}). */
export function extractEvidence(sentence: string): { text: string; evidence: boolean }[] {
  const out: { text: string; evidence: boolean }[] = [];
  const re = /[“"']([^"'“”]{4,200}?)["”']/g;
  let lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(sentence)) !== null) {
    if (m.index > lastIndex) {
      out.push({ text: sentence.slice(lastIndex, m.index), evidence: false });
    }
    out.push({ text: m[1], evidence: true });
    lastIndex = m.index + m[0].length;
  }
  if (lastIndex < sentence.length) {
    out.push({ text: sentence.slice(lastIndex), evidence: false });
  }
  return out.length > 0 ? out : [{ text: sentence, evidence: false }];
}

