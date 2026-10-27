import { useEffect, useState } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  Handle,
  Position,
  type Node,
  type Edge,
  type NodeProps,
  useNodesState,
  useEdgesState,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Loader2, Network, RefreshCw } from "lucide-react";
import { api, type ClusterArticle, type ClusterNode } from "../../lib/api";
import { startClusterBuild, useClusterTask } from "../../lib/bgTasks";

// ── Custom node types ─────────────────────────────────────────────────────────

function RootNode({ data }: NodeProps) {
  return (
    <>
      <div
        style={{
          background: "var(--color-ink-800)",
          color: "white",
          borderRadius: "var(--radius-sm)",
          fontSize: "13px",
          fontWeight: 700,
          padding: "8px 20px",
          whiteSpace: "nowrap",
        }}
      >
        {data.label as string}
      </div>
      <Handle type="source" position={Position.Bottom} style={{ visibility: "hidden" }} />
    </>
  );
}

function ClusterCard({ data }: NodeProps) {
  const articles = data.articles as ClusterArticle[];
  return (
    <>
      <Handle type="target" position={Position.Top} style={{ visibility: "hidden" }} />
      <div
        style={{
          background: "white",
          border: "1.5px solid var(--color-accent-300)",
          borderRadius: "var(--radius-sm)",
          width: 200,
          overflow: "hidden",
        }}
      >
        {/* Cluster label */}
        <div
          style={{
            background: "var(--color-accent-50)",
            padding: "6px 10px",
            fontSize: "11px",
            fontWeight: 700,
            color: "var(--color-accent-800)",
            borderBottom: "1px solid var(--color-accent-200)",
          }}
        >
          {data.label as string}
        </div>
        {/* Article list */}
        <div style={{ padding: "4px 0" }}>
          {articles.map((a) => (
            <div
              key={a.id}
              style={{
                fontSize: "10px",
                color: "var(--color-ink-600)",
                padding: "2px 10px",
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
              }}
              title={a.title}
            >
              {a.title}
            </div>
          ))}
        </div>
      </div>
    </>
  );
}

const NODE_TYPES = { root: RootNode, cluster: ClusterCard };

// ── Layout ────────────────────────────────────────────────────────────────────

function buildGraph(clusters: ClusterNode[]): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];

  const CARD_WIDTH = 200;
  const GAP = 24;
  const STEP = CARD_WIDTH + GAP;
  const totalWidth = clusters.length * STEP - GAP;

  nodes.push({
    id: "root",
    type: "root",
    position: { x: totalWidth / 2 - 60, y: 0 },
    data: { label: "Your Literature" },
  });

  clusters.forEach((cluster, ci) => {
    const clusterId = `cluster-${cluster.id}`;
    nodes.push({
      id: clusterId,
      type: "cluster",
      position: { x: ci * STEP, y: 90 },
      data: { label: cluster.label, articles: cluster.articles },
    });
    edges.push({
      id: `root-${clusterId}`,
      source: "root",
      target: clusterId,
      style: { stroke: "var(--color-paper-400)", strokeWidth: 1.5 },
    });
  });

  return { nodes, edges };
}

// ── Component ─────────────────────────────────────────────────────────────────

export function ClustersTab() {
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const task = useClusterTask();
  const loading = task.running;
  const progress = task.progress;
  const error = task.error;
  const hasData = nodes.length > 0;

  // Whenever a fresh result lands in the store (whether the user kicked it
  // off from this tab or the run completed while they were elsewhere),
  // rebuild the graph. The dependency on `task.finishedAt` makes sure we
  // only react to actual completions, not every store update.
  // Number of clusters: null = automatic. Remembered per project.
  const [k, setK] = useState<number | null>(null);
  useEffect(() => {
    api.getSettings()
      .then((s) => {
        const v = s["clusters.k"];
        setK(typeof v === "number" ? v : null);
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (task.result) {
      const { nodes: n, edges: e } = buildGraph(task.result.clusters);
      setNodes(n);
      setEdges(e);
    }
  }, [task.finishedAt, task.result, setNodes, setEdges]);

  const run = () => startClusterBuild(k);
  const result = task.result;

  if (!hasData && !loading) {
    return (
      <div className="h-full flex flex-col items-center justify-center gap-4 p-8 text-center">
        <Network size={36} strokeWidth={1.25} className="text-[var(--color-paper-300)]" />
        <div>
          <p className="text-[14px] font-semibold text-[var(--color-ink-800)]">Cluster your literature</p>
          <p className="text-[12px] text-[var(--color-paper-500)] mt-1 max-w-xs">
            Groups articles by semantic similarity using K-means on their embeddings, then labels each cluster with the LLM.
            Requires indexed and embedded PDFs.
          </p>
        </div>
        <ClusterCountInput value={k} onChange={setK} />
        {error && <p className="text-[12px] text-[var(--color-error-600)]">{error}</p>}
        <button
          onClick={run}
          className="flex items-center gap-1.5 px-4 py-2 rounded-[var(--radius-sm)] bg-[var(--color-ink-800)] text-white text-[12px] font-semibold hover:bg-[var(--color-ink-700)] transition-paper"
        >
          <Network size={13} strokeWidth={2} /> Build clusters
        </button>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="h-full flex flex-col items-center justify-center gap-3">
        <Loader2 size={24} className="animate-spin text-[var(--color-accent-500)]" />
        <p className="text-[13px] text-[var(--color-paper-600)]">
          {progress
            ? progress.phase === "labelling"
              ? `Labelling cluster ${progress.done + 1} of ${progress.total}…`
              : progress.phase === "clustering"
              ? "Running K-means…"
              : `${progress.phase}…`
            : "Starting…"}
        </p>
      </div>
    );
  }

  return (
    <div className="h-full relative">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={NODE_TYPES}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.2}
        maxZoom={2}
        nodesDraggable
      >
        <Background color="var(--color-paper-200)" gap={20} />
        <Controls />
      </ReactFlow>
      <div className="absolute top-3 right-3 z-10 flex items-center gap-2 px-3 py-1.5 rounded-[var(--radius-sm)] bg-white border border-[var(--color-paper-300)] shadow-sm">
        {result && (
          <span className="text-[11px] text-[var(--color-paper-500)]">
            {result.k_used} clusters from {result.articles} papers
            {result.k_requested === null
              ? " (automatic)"
              : result.k_requested !== result.k_used
              ? ` (you asked for ${result.k_requested})`
              : ""}
          </span>
        )}
        <ClusterCountInput value={k} onChange={setK} compact />
        <button
          onClick={run}
          className="flex items-center gap-1.5 px-2 py-1 rounded-[var(--radius-xs)] text-[11px] font-medium text-[var(--color-paper-600)] hover:text-[var(--color-ink-800)] hover:bg-[var(--color-paper-100)] transition-paper"
          title="Re-run clustering with this number. Cluster labels are regenerated."
        >
          <RefreshCw size={11} strokeWidth={2} /> Rebuild
        </button>
      </div>
      {error && (
        <div className="absolute bottom-3 left-1/2 -translate-x-1/2 z-10 px-4 py-2 rounded-[var(--radius-sm)] bg-white border border-[var(--color-error-200)] text-[12px] text-[var(--color-error-600)] shadow-sm">
          {error}
        </div>
      )}
    </div>
  );
}

/** "Automatic" or a chosen number of clusters (2–50; the build keeps it at or
 *  below the number of papers). */
function ClusterCountInput({
  value,
  onChange,
  compact = false,
}: {
  value: number | null;
  onChange: (v: number | null) => void;
  compact?: boolean;
}) {
  const auto = value === null;
  return (
    <label className="flex items-center gap-1.5 text-[11px] text-[var(--color-paper-600)]">
      {compact ? "Clusters" : "Number of clusters"}
      <select
        value={auto ? "auto" : "set"}
        onChange={(e) => onChange(e.target.value === "auto" ? null : 8)}
        className="px-1.5 py-0.5 rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white text-[11px]"
      >
        <option value="auto">Automatic</option>
        <option value="set">Choose…</option>
      </select>
      {!auto && (
        <input
          type="number"
          min={2}
          max={50}
          value={value}
          onChange={(e) => {
            const n = Number.parseInt(e.target.value, 10);
            if (Number.isFinite(n)) onChange(Math.min(50, Math.max(2, n)));
          }}
          className="w-14 px-1.5 py-0.5 rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white text-[11px] tabular-nums"
          aria-label="Number of clusters"
        />
      )}
    </label>
  );
}
