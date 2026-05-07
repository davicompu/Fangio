import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  GitMerge,
  Loader2,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { api, type ArticleRow, type DedupCluster } from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { MarkStepComplete } from "../../components/MarkStepComplete";

interface Props {
  refreshKey: number;
  onChanged: () => void;
  manualComplete?: boolean;
}

interface ResolvedCluster {
  cluster: DedupCluster;
  members: ArticleRow[];
}

export function DedupReview({ refreshKey, onChanged, manualComplete }: Props) {
  const [clusters, setClusters] = useState<ResolvedCluster[] | null>(null);
  const [busy, setBusy] = useState<number | null>(null);

  useEffect(() => {
    (async () => {
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
    })();
  }, [refreshKey]);

  const merge = async (idx: number, members: number[]) => {
    setBusy(idx);
    try {
      await api.mergeDedupCluster(members);
      setClusters((cs) => (cs ? cs.filter((_, i) => i !== idx) : cs));
      onChanged();
    } finally {
      setBusy(null);
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

  return (
    <div className="space-y-5">
      <div className="flex items-center gap-3">
        <Sparkles size={14} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
        <p className="text-[13px] text-[var(--color-paper-700)]">
          <strong>{clusters.length}</strong> potential duplicate cluster
          {clusters.length === 1 ? "" : "s"} found by title-and-year fuzzy matching. Review each
          and merge or keep separate. Merging keeps the lowest-numbered record and marks the
          rest as duplicates.
        </p>
      </div>

      <AnimatePresence mode="popLayout">
        {clusters.map((c, idx) => (
          <motion.div
            key={c.cluster.members.join("-")}
            layout
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.97 }}
            transition={{ duration: 0.3 }}
          >
            <Card className="p-5">
              <div className="flex items-start justify-between mb-4">
                <div>
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
                </div>
                <div className="flex gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setClusters((cs) => (cs ? cs.filter((_, i) => i !== idx) : cs))}
                  >
                    Keep separate
                  </Button>
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => merge(idx, c.cluster.members)}
                    disabled={busy === idx}
                  >
                    {busy === idx ? (
                      <Loader2 size={13} className="animate-spin" />
                    ) : (
                      <GitMerge size={13} strokeWidth={2} />
                    )}
                    Merge
                  </Button>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                {c.members.map((m, i) => (
                  <div
                    key={m.id}
                    className="p-3.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-200)]"
                  >
                    <div className="flex items-baseline gap-2 mb-1">
                      <span className="font-mono text-[10px] text-[var(--color-paper-500)] tabular-nums">
                        #{m.id}
                      </span>
                      {i === 0 && (
                        <span className="text-[9px] font-semibold uppercase tracking-wider px-1.5 py-0.5 rounded-sm bg-[var(--color-ink-700)] text-[var(--color-paper-50)]">
                          Keeper
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
                  </div>
                ))}
              </div>
            </Card>
          </motion.div>
        ))}
      </AnimatePresence>
      <MarkStepComplete
        stage="assembling"
        tab="dedup"
        isComplete={!!manualComplete}
        onChanged={onChanged}
      />
    </div>
  );
}
