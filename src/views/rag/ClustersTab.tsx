import { useCallback, useState } from "react";
import { listen } from "@tauri-apps/api/event";
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
import { api, type ClusterArticle, type ClusterNode, type ClusterProgress } from "../../lib/api";

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
  const [loading, setLoading] = useState(false);
  const [progress, setProgress] = useState<ClusterProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [hasData, setHasData] = useState(false);

  const run = useCallback(async () => {
    setLoading(true);
    setError(null);
    setProgress(null);

    const unlisten = await listen<ClusterProgress>("clusters:progress", (e) => {
      setProgress(e.payload);
    });

    try {
      const clusters = await api.clusterCorpus();
      const { nodes: n, edges: e } = buildGraph(clusters);
      setNodes(n);
      setEdges(e);
      setHasData(true);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      unlisten();
      setLoading(false);
    }
  }, [setNodes, setEdges]);

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
      <button
        onClick={run}
        className="absolute top-3 right-3 z-10 flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] bg-white border border-[var(--color-paper-300)] text-[11px] font-medium text-[var(--color-paper-600)] hover:text-[var(--color-ink-800)] shadow-sm transition-paper"
      >
        <RefreshCw size={11} strokeWidth={2} /> Rebuild
      </button>
      {error && (
        <div className="absolute bottom-3 left-1/2 -translate-x-1/2 z-10 px-4 py-2 rounded-[var(--radius-sm)] bg-white border border-[var(--color-error-200)] text-[12px] text-[var(--color-error-600)] shadow-sm">
          {error}
        </div>
      )}
    </div>
  );
}
