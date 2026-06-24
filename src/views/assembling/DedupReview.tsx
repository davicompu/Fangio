import { useEffect, useMemo, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Check,
  Loader2,
  ShieldCheck,
  Sparkles,
  Trash2,
  X,
} from "lucide-react";
import { api, type ArticleRow, type DedupCluster } from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { cn } from "../../lib/utils";

interface Props {
  refreshKey: number;
  onChanged: () => void;
}

interface ResolvedCluster {
  cluster: DedupCluster;
  members: ArticleRow[];
}

type Decision =
  | { kind: "keep"; keeperId: number }
  | { kind: "not_duplicates" };

const signatureFor = (members: number[]) =>
  [...members].sort((a, b) => a - b).join("-");

export function DedupReview({ refreshKey, onChanged }: Props) {
  const [clusters, setClusters] = useState<ResolvedCluster[] | null>(null);
  const [decisions, setDecisions] = useState<Record<string, Decision>>({});
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reload = async () => {
    setClusters(null);
    const [rawClusters, articles] = await Promise.all([
      api.listDedupClusters(),
      api.listArticles(),
    ]);
    const byId = new Map(articles.map((a) => [a.id, a]));
    const resolved = rawClusters
      .map((c) => ({
        cluster: c,
        members: c.members.map((id) => byId.get(id)!).filter(Boolean),
      }))
      .filter((c) => c.members.length >= 2);
    setClusters(resolved);
    setDecisions({});
  };

  useEffect(() => {
    reload().catch((e) => setError(e?.toString() ?? String(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshKey]);

  const setKeeper = (signature: string, keeperId: number) =>
    setDecisions((d) => ({ ...d, [signature]: { kind: "keep", keeperId } }));

  const setNotDuplicates = (signature: string) =>
    setDecisions((d) => ({ ...d, [signature]: { kind: "not_duplicates" } }));

  const clearDecision = (signature: string) =>
    setDecisions((d) => {
      const next = { ...d };
      delete next[signature];
      return next;
    });

  const counts = useMemo(() => {
    let keep = 0;
    let notDup = 0;
    let duplicatesToRemove = 0;
    if (!clusters) return { keep, notDup, duplicatesToRemove, total: 0 };
    for (const c of clusters) {
      const sig = signatureFor(c.cluster.members);
      const d = decisions[sig];
      if (!d) continue;
      if (d.kind === "keep") {
        keep += 1;
        duplicatesToRemove += c.members.length - 1;
      } else {
        notDup += 1;
      }
    }
    return { keep, notDup, duplicatesToRemove, total: keep + notDup };
  }, [clusters, decisions]);

  const applyAll = async () => {
    if (!clusters || counts.total === 0) return;
    setApplying(true);
    setError(null);
    try {
      const decided = clusters
        .map((c) => ({ cluster: c, decision: decisions[signatureFor(c.cluster.members)] }))
        .filter((entry): entry is { cluster: ResolvedCluster; decision: Decision } => Boolean(entry.decision));

      // Run sequentially so the SQLite writes serialise cleanly. The set is small;
      // parallelism here would only invite race conditions for marginal speed.
      for (const { cluster, decision } of decided) {
        if (decision.kind === "keep") {
          await api.mergeDedupCluster(cluster.cluster.members, decision.keeperId);
        } else {
          await api.keepDedupClusterSeparate(cluster.cluster.members);
        }
      }

      // Drop the resolved clusters from view + clear their decisions, then notify
      // the parent so progress signals refresh.
      const resolvedSigs = new Set(decided.map((e) => signatureFor(e.cluster.cluster.members)));
      setClusters((cs) => (cs ? cs.filter((c) => !resolvedSigs.has(signatureFor(c.cluster.members))) : cs));
      setDecisions((d) => {
        const next = { ...d };
        for (const sig of resolvedSigs) delete next[sig];
        return next;
      });
      onChanged();
    } catch (e: any) {
      // A write may have partially applied — re-fetch from the backend so the UI
      // reflects what actually committed and the user can't double-merge.
      setError(`Some merges didn't apply (${e?.message ?? e}). Re-syncing the remaining clusters…`);
      await reload().catch(() => {});
      onChanged();
    } finally {
      setApplying(false);
    }
  };

  if (clusters === null) {
    return (
      <div className="flex items-center gap-2 text-[13px] text-[var(--color-paper-500)]">
        <Loader2 size={14} className="animate-spin" /> Scanning for duplicates…
      </div>
    );
  }

  if (clusters.length === 0) {
    return (
      <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-12 text-center">
        <ShieldCheck
          size={28}
          strokeWidth={1.5}
          className="mx-auto mb-3 text-[var(--color-ok-500)]"
        />
        <p className="text-sm text-[var(--color-ink-800)] font-medium mb-1">
          No duplicate clusters need review
        </p>
        <p className="text-[13px] text-[var(--color-paper-500)] max-w-md mx-auto leading-relaxed">
          DOI duplicates are skipped automatically on import. The remaining records appear unique
          on title and year. If you import more files later, come back and re-check.
        </p>
      </div>
    );
  }

  const pending = clusters.length - counts.total;

  return (
    <div className="space-y-5 pb-28">
      {error && (
        <div className="px-4 py-2.5 rounded-[var(--radius-sm)] bg-[var(--color-warn-50)] border border-[var(--color-warn-300)] text-[12px] text-[var(--color-warn-700)]">
          {error}
        </div>
      )}
      <div className="flex items-start gap-3">
        <Sparkles size={14} strokeWidth={1.75} className="text-[var(--color-accent-500)] mt-0.5 shrink-0" />
        <p className="text-[13px] text-[var(--color-paper-700)] leading-relaxed">
          <strong>{clusters.length}</strong> potential duplicate cluster
          {clusters.length === 1 ? "" : "s"} found by DOI or title fuzzy matching. For each cluster,
          pick which record to <strong>keep</strong> (others are flagged as duplicates), or mark it
          as <strong>Not duplicates</strong> if the records are actually different articles. Then
          press <strong>Remove duplicates</strong> at the bottom to apply all decisions at once.
        </p>
      </div>

      <AnimatePresence mode="popLayout">
        {clusters.map((c) => {
          const sig = signatureFor(c.cluster.members);
          const decision = decisions[sig];
          const keeperId = decision?.kind === "keep" ? decision.keeperId : undefined;
          const isNotDup = decision?.kind === "not_duplicates";
          return (
            <motion.div
              key={sig}
              layout
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.97 }}
              transition={{ duration: 0.3 }}
            >
              <Card className={cn("p-5", isNotDup && "bg-[var(--color-paper-100)]")}>
                <div className="flex items-start justify-between mb-4 gap-4">
                  <div className="flex-1 min-w-0">
                    <div className="eyebrow text-[var(--color-paper-500)]">
                      Cluster · {c.cluster.kind === "doi-exact" ? "DOI exact match" : "Title fuzzy"}
                      {c.cluster.kind === "title-fuzzy" && (
                        <span className="ml-2 font-mono text-[var(--color-paper-600)]">
                          {(c.cluster.confidence * 100).toFixed(1)}% match
                        </span>
                      )}
                    </div>
                    <h3 className="font-serif text-[18px] tracking-tight text-[var(--color-ink-900)] mt-1">
                      {c.members.length} candidate records
                    </h3>
                    <p className="text-[12px] text-[var(--color-paper-500)] mt-1">
                      {keeperId !== undefined
                        ? `Will keep #${keeperId} · ${c.members.length - 1} marked as duplicate${c.members.length - 1 === 1 ? "" : "s"}`
                        : isNotDup
                        ? "Marked Not duplicates · cluster will be suppressed"
                        : "Click a record to mark it as the keeper, or Not duplicates if these are different articles"}
                    </p>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    {decision && (
                      <button
                        type="button"
                        onClick={() => clearDecision(sig)}
                        disabled={applying}
                        className="inline-flex items-center gap-1 px-2 py-1 rounded-[var(--radius-xs)] text-[11px] text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] hover:bg-[var(--color-paper-200)] transition-paper disabled:opacity-50"
                        title="Clear this cluster's decision"
                      >
                        <X size={11} strokeWidth={2} /> Reset
                      </button>
                    )}
                    <Button
                      variant={isNotDup ? "primary" : "ghost"}
                      size="sm"
                      onClick={() => (isNotDup ? clearDecision(sig) : setNotDuplicates(sig))}
                      disabled={applying}
                    >
                      {isNotDup ? <Check size={13} strokeWidth={2} /> : null}
                      Not duplicates
                    </Button>
                  </div>
                </div>

                <div className={cn("grid grid-cols-2 gap-3", isNotDup && "opacity-50")}>
                  {c.members.map((m) => {
                    const isKeeper = keeperId === m.id;
                    const willBeDuplicate = keeperId !== undefined && !isKeeper;
                    return (
                      <button
                        key={m.id}
                        type="button"
                        onClick={() => setKeeper(sig, m.id)}
                        disabled={applying || isNotDup}
                        className={cn(
                          "no-drag text-left p-3.5 rounded-[var(--radius-sm)] border transition-paper",
                          isKeeper
                            ? "bg-white border-[var(--color-ok-500)] ring-2 ring-[var(--color-ok-500)]/30 shadow-[var(--shadow-paper)]"
                            : willBeDuplicate
                            ? "bg-[var(--color-paper-100)] border-[var(--color-paper-200)] opacity-60"
                            : "bg-[var(--color-paper-100)] border-[var(--color-paper-200)] hover:border-[var(--color-paper-400)] hover:bg-white",
                          (applying || isNotDup) && "cursor-not-allowed",
                        )}
                      >
                        <div className="flex items-baseline gap-2 mb-1">
                          <span className="font-mono text-[10px] text-[var(--color-paper-500)] tabular-nums">
                            #{m.id}
                          </span>
                          {isKeeper ? (
                            <span className="inline-flex items-center gap-1 text-[9px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded-sm bg-[var(--color-ok-500)] text-white">
                              <Check size={9} strokeWidth={2.5} />
                              Keep this
                            </span>
                          ) : willBeDuplicate ? (
                            <span className="text-[9px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded-sm bg-[var(--color-paper-300)] text-[var(--color-paper-700)]">
                              Duplicate
                            </span>
                          ) : (
                            <span className="text-[9px] font-medium uppercase tracking-wider px-1.5 py-0.5 rounded-sm border border-[var(--color-paper-300)] text-[var(--color-paper-500)]">
                              Click to keep
                            </span>
                          )}
                          {m.year && (
                            <span className="font-mono text-[10px] text-[var(--color-paper-500)] ml-auto">
                              {m.year}
                            </span>
                          )}
                        </div>
                        <h4 className="font-serif text-[13px] leading-snug text-[var(--color-ink-900)] line-clamp-2 mb-1">
                          {m.title}
                        </h4>
                        {m.authors && (
                          <p className="text-[11px] text-[var(--color-paper-600)] line-clamp-1">
                            {m.authors}
                          </p>
                        )}
                        {m.journal && (
                          <p className="text-[11px] italic text-[var(--color-paper-500)] line-clamp-1 mt-0.5">
                            {m.journal}
                          </p>
                        )}
                        {m.doi && (
                          <p className="font-mono text-[10px] text-[var(--color-paper-500)] mt-1.5 truncate">
                            {m.doi}
                          </p>
                        )}
                      </button>
                    );
                  })}
                </div>
              </Card>
            </motion.div>
          );
        })}
      </AnimatePresence>

      {/* Sticky action bar — apply all decisions at once */}
      <div className="fixed bottom-9 left-0 right-0 z-30 px-6 xl:px-12 pointer-events-none">
        <div className="max-w-[calc(100%-260px)] xl:max-w-[calc(100%-320px)] mx-auto pointer-events-auto">
          <div className="flex items-center justify-between gap-4 px-4 py-3 rounded-[var(--radius-md)] bg-white/95 backdrop-blur-sm border border-[var(--color-paper-300)] shadow-[var(--shadow-paper-lg)]">
            <div className="flex items-center gap-3 text-[12px] text-[var(--color-paper-700)] min-w-0">
              <span className="tabular-nums">
                <strong className="text-[var(--color-ink-900)]">{counts.total}</strong> of {clusters.length} cluster{clusters.length === 1 ? "" : "s"} decided
              </span>
              {counts.keep > 0 && (
                <span className="tabular-nums text-[var(--color-paper-600)]">
                  · {counts.duplicatesToRemove} record{counts.duplicatesToRemove === 1 ? "" : "s"} to remove
                </span>
              )}
              {counts.notDup > 0 && (
                <span className="tabular-nums text-[var(--color-paper-600)]">
                  · {counts.notDup} kept separate
                </span>
              )}
              {pending > 0 && (
                <span className="text-[var(--color-paper-500)]">
                  · {pending} pending
                </span>
              )}
            </div>
            <Button
              variant="primary"
              size="sm"
              onClick={applyAll}
              disabled={counts.total === 0 || applying}
            >
              {applying ? <Loader2 size={13} className="animate-spin" /> : <Trash2 size={13} strokeWidth={2} />}
              {applying ? "Applying…" : "Remove duplicates"}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
