import { useCallback, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  ReactFlow,
  Background,
  Controls,
  type Node,
  type Edge,
  useNodesState,
  useEdgesState,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Loader2, Network, RefreshCw } from "lucide-react";
import { api, type ClusterNode, type ClusterProgress } from "../../lib/api";

// Build react-flow nodes and edges from cluster data
function buildGraph(clusters: ClusterNode[]): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];

  // Root node
  nodes.push({
    id: "root",
    type: "default",
    position: { x: 0, y: 0 },
    data: { label: "Your Literature" },
    style: {
      background: "var(--color-ink-800)",
      color: "white",
      border: "none",
      borderRadius: "var(--radius-sm)",
      fontSize: "13px",
      fontWeight: 600,
      padding: "8px 16px",
    },
  });

  const clusterSpacing = Math.max(220, (clusters.reduce((m, c) => Math.max(m, c.articles.length), 0)) * 160);

  clusters.forEach((cluster, ci) => {
    const clusterX = (ci - (clusters.length - 1) / 2) * clusterSpacing;
    const clusterId = `cluster-${cluster.id}`;

    nodes.push({
      id: clusterId,
      type: "default",
      position: { x: clusterX, y: 140 },
      data: { label: cluster.label },
      style: {
        background: "var(--color-accent-50)",
        border: "1px solid var(--color-accent-300)",
        borderRadius: "var(--radius-sm)",
        fontSize: "12px",
        fontWeight: 600,
        color: "var(--color-accent-800)",
        padding: "6px 12px",
        maxWidth: 180,
        textAlign: "center",
      },
    });

    edges.push({
      id: `root-${clusterId}`,
      source: "root",
      target: clusterId,
      style: { stroke: "var(--color-paper-300)", strokeWidth: 1.5 },
    });

    cluster.articles.forEach((article, ai) => {
      const articleId = `article-${article.id}`;
      const articleX = clusterX + (ai - (cluster.articles.length - 1) / 2) * 160;

      nodes.push({
        id: articleId,
        type: "default",
        position: { x: articleX, y: 300 },
        data: { label: article.title.length > 50 ? article.title.slice(0, 50) + "…" : article.title },
        style: {
          background: "white",
          border: "1px solid var(--color-paper-200)",
          borderRadius: "var(--radius-xs)",
          fontSize: "10px",
          color: "var(--color-ink-700)",
          padding: "4px 8px",
          maxWidth: 150,
          textAlign: "center",
        },
      });

      edges.push({
        id: `${clusterId}-${articleId}`,
        source: clusterId,
        target: articleId,
        style: { stroke: "var(--color-paper-200)", strokeWidth: 1 },
      });
    });
  });

  return { nodes, edges };
}

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
        fitView
        fitViewOptions={{ padding: 0.15 }}
        minZoom={0.2}
        maxZoom={2}
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
    </div>
  );
}
