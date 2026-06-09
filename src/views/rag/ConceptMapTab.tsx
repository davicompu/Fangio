import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ForceGraph2D, { type ForceGraphMethods } from "react-force-graph-2d";
// react-force-graph-2d ships with d3-force-3d; import the X/Y centering
// helpers from the same package so versions stay in lockstep.
// @ts-expect-error — no shipped TS types for d3-force-3d, but the runtime API matches d3-force.
import { forceX, forceY } from "d3-force-3d";
import { Loader2, Network, RefreshCw } from "lucide-react";
import { api, type ArticleRow, type GlossaryTerm } from "../../lib/api";

// ── Graph types ─────────────────────────────────────────────────────────────

type NodeKind = "article" | "term";

interface GraphNode {
  id: string;
  kind: NodeKind;
  label: string;
  /** number of edges incident to this node — used to size it. */
  degree: number;
  /** Article-only — its primary key, painted inside the square. */
  articleId?: number;
  /** Optional metadata for tooltips and click handlers. */
  pdfPath?: string;
  authors?: string | null;
  year?: number | null;
  /** Term-only fields — populated when kind === "term". */
  definition?: string;
  aliases?: string[];
  /** Force-graph mutates these in place during simulation. */
  x?: number;
  y?: number;
}

interface GraphLink {
  source: string;
  target: string;
}

// ── Build the bipartite graph from glossary + articles ──────────────────────

function buildGraph(
  terms: GlossaryTerm[],
  articles: ArticleRow[],
  pdfPaths: Map<number, string>,
): { nodes: GraphNode[]; links: GraphLink[] } {
  const articleById = new Map(articles.map((a) => [a.id, a]));
  const nodes: GraphNode[] = [];
  const links: GraphLink[] = [];

  // Track degree counts so we can size nodes by connectivity.
  const degree = new Map<string, number>();
  const bump = (id: string) => degree.set(id, (degree.get(id) ?? 0) + 1);

  // Articles that are referenced by at least one term get a node. Articles
  // that no term mentions would be isolated dots — we filter them out.
  const referencedArticleIds = new Set<number>();
  const termArticlePairs: { termId: string; articleId: string }[] = [];

  for (const term of terms) {
    const termId = `t:${term.id}`;
    let articleIds: number[] = [];
    try {
      articleIds = JSON.parse(term.article_ids);
    } catch {
      continue;
    }
    for (const aid of articleIds) {
      if (!articleById.has(aid)) continue; // term references an article that's no longer in the corpus
      referencedArticleIds.add(aid);
      const articleId = `a:${aid}`;
      termArticlePairs.push({ termId, articleId });
      bump(termId);
      bump(articleId);
    }
  }

  for (const term of terms) {
    const termId = `t:${term.id}`;
    if (!degree.has(termId)) continue; // skip terms with no resolvable articles
    let aliases: string[] = [];
    try { aliases = JSON.parse(term.aliases || "[]"); } catch { /* noop */ }
    nodes.push({
      id: termId,
      kind: "term",
      label: term.term,
      degree: degree.get(termId) ?? 0,
      definition: term.definition,
      aliases,
    });
  }
  for (const aid of referencedArticleIds) {
    const a = articleById.get(aid)!;
    nodes.push({
      id: `a:${aid}`,
      kind: "article",
      label: a.title,
      degree: degree.get(`a:${aid}`) ?? 0,
      articleId: aid,
      pdfPath: pdfPaths.get(aid),
      authors: a.authors,
      year: a.year,
    });
  }
  for (const { termId, articleId } of termArticlePairs) {
    links.push({ source: termId, target: articleId });
  }

  return { nodes, links };
}

// ── Custom node painter ─────────────────────────────────────────────────────
//
// react-force-graph-2d gives us a canvas context per node so we can draw
// distinct shapes: circles for articles, rounded squares for terms.

function paintNode(
  node: GraphNode,
  ctx: CanvasRenderingContext2D,
  globalScale: number,
  highlighted: Set<string>,
  hovered: string | null,
  selected: string | null,
) {
  const isArticle = node.kind === "article";
  // Article squares are sized to fit a 1–3 digit ID; term circles stay
  // smaller and grow with degree to surface hub concepts visually.
  const base = isArticle ? 12 : 5;
  const size = base + Math.min(8, Math.sqrt(node.degree) * 1.4);
  const x = node.x ?? 0;
  const y = node.y ?? 0;
  const dim = highlighted.size > 0 && !highlighted.has(node.id);
  const isHovered = hovered === node.id;
  const isSelected = selected === node.id;

  ctx.lineWidth = isSelected ? 3 : isHovered ? 2.5 : 1.25;
  ctx.strokeStyle = isSelected
    ? "rgba(15, 23, 42, 1)"
    : isHovered
    ? "rgba(15, 23, 42, 0.9)"
    : isArticle
    ? "rgba(15, 118, 110, 0.9)"
    : "rgba(107, 70, 193, 0.9)";
  ctx.fillStyle = isArticle
    ? dim ? "rgba(153, 246, 228, 0.4)" : "rgba(94, 234, 212, 0.92)"
    : dim ? "rgba(196, 181, 253, 0.35)" : "rgba(167, 139, 250, 0.85)";

  if (isArticle) {
    // Rounded square
    const r = 3;
    const half = size;
    ctx.beginPath();
    ctx.moveTo(x - half + r, y - half);
    ctx.lineTo(x + half - r, y - half);
    ctx.quadraticCurveTo(x + half, y - half, x + half, y - half + r);
    ctx.lineTo(x + half, y + half - r);
    ctx.quadraticCurveTo(x + half, y + half, x + half - r, y + half);
    ctx.lineTo(x - half + r, y + half);
    ctx.quadraticCurveTo(x - half, y + half, x - half, y + half - r);
    ctx.lineTo(x - half, y - half + r);
    ctx.quadraticCurveTo(x - half, y - half, x - half + r, y - half);
    ctx.closePath();
  } else {
    ctx.beginPath();
    ctx.arc(x, y, size, 0, Math.PI * 2);
    ctx.closePath();
  }
  ctx.fill();
  ctx.stroke();

  // Article squares: paint the article id inside the square.
  if (isArticle && node.articleId != null) {
    const fontSize = Math.max(9, size * 0.95);
    ctx.font = `700 ${fontSize}px -apple-system, system-ui, sans-serif`;
    ctx.fillStyle = dim ? "rgba(15, 118, 110, 0.55)" : "rgba(6, 78, 59, 1)";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(String(node.articleId), x, y + 0.5);
  }

  // External labels:
  //  - articles never show their title in the canvas (the # inside the square
  //    is the identifier; the full title is on hover/click)
  //  - terms show their label for hubs (degree ≥ 4), on hover, on select, or
  //    when the user zooms in
  const showLabel = !isArticle && (isHovered || isSelected || globalScale > 1.4 || node.degree >= 4);
  if (showLabel) {
    const fontSize = Math.max(10, 12 / globalScale);
    ctx.font = `${isHovered || isSelected ? "600" : "500"} ${fontSize}px -apple-system, system-ui, sans-serif`;
    ctx.fillStyle = dim ? "rgba(100, 116, 139, 0.6)" : "rgba(15, 23, 42, 0.92)";
    ctx.textAlign = "center";
    ctx.textBaseline = "top";
    const truncated = node.label.length > 36 ? node.label.slice(0, 34) + "…" : node.label;
    ctx.fillText(truncated, x, y + size + 3);
  }
}

// ── Component ───────────────────────────────────────────────────────────────

export function ConceptMapTab() {
  const [graph, setGraph] = useState<{ nodes: GraphNode[]; links: GraphLink[] }>({ nodes: [], links: [] });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hovered, setHovered] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [size, setSize] = useState({ width: 800, height: 600 });
  const containerRef = useRef<HTMLDivElement | null>(null);
  const fgRef = useRef<ForceGraphMethods<GraphNode, GraphLink> | undefined>(undefined);

  // Tune the d3-force simulation so the bipartite graph spreads out without
  // disconnected components flying off-screen.
  //  - charge: repulsion between every pair of nodes
  //  - link distance: target distance for edges
  //  - x/y: pull every node gently toward the origin so isolated subgraphs
  //         stay nearby rather than drifting to infinity (this is the key fix
  //         for "lonely" articles ending up far from the main cluster)
  useEffect(() => {
    const fg = fgRef.current;
    if (!fg) return;
    const charge = fg.d3Force("charge");
    if (charge && "strength" in charge) (charge as unknown as { strength: (s: number) => void }).strength(-180);
    const link = fg.d3Force("link");
    if (link && "distance" in link) (link as unknown as { distance: (d: number) => void }).distance(60);
    fg.d3Force("x", forceX(0).strength(0.08));
    fg.d3Force("y", forceY(0).strength(0.08));
    fg.d3ReheatSimulation();
  }, [graph]);

  // Keep the canvas sized to the container (the parent flex/grid
  // doesn't propagate width to react-force-graph automatically).
  useEffect(() => {
    if (!containerRef.current) return;
    const el = containerRef.current;
    const ro = new ResizeObserver(() => {
      setSize({ width: el.clientWidth, height: el.clientHeight });
    });
    ro.observe(el);
    setSize({ width: el.clientWidth, height: el.clientHeight });
    return () => ro.disconnect();
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [terms, articles] = await Promise.all([
        api.listGlossaryTerms(),
        api.listArticles(),
      ]);
      const ids = articles.map((a) => a.id);
      const pdfMap = ids.length > 0 ? await api.pdfPathsForArticles(ids).catch(() => ({} as Record<number, string>)) : {};
      const pdfs = new Map<number, string>();
      for (const [k, v] of Object.entries(pdfMap)) pdfs.set(Number(k), v);
      setGraph(buildGraph(terms, articles, pdfs));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  // When hovering a node, highlight it + its direct neighbours so the user
  // can read the bipartite slice that node participates in.
  const highlighted = useMemo(() => {
    if (!hovered) return new Set<string>();
    const set = new Set<string>([hovered]);
    for (const link of graph.links) {
      const s = typeof link.source === "string" ? link.source : (link.source as GraphNode).id;
      const t = typeof link.target === "string" ? link.target : (link.target as GraphNode).id;
      if (s === hovered) set.add(t);
      if (t === hovered) set.add(s);
    }
    return set;
  }, [hovered, graph.links]);

  const counts = useMemo(() => {
    let articles = 0;
    let terms = 0;
    for (const n of graph.nodes) {
      if (n.kind === "article") articles++;
      else terms++;
    }
    return { articles, terms, links: graph.links.length };
  }, [graph]);

  return (
    <div ref={containerRef} className="h-full relative bg-[var(--color-paper-50)]">
      {loading && graph.nodes.length === 0 && (
        <div className="absolute inset-0 flex flex-col items-center justify-center gap-3">
          <Loader2 size={24} className="animate-spin text-[var(--color-accent-500)]" />
          <p className="text-[13px] text-[var(--color-paper-600)]">Loading concept map…</p>
        </div>
      )}

      {!loading && graph.nodes.length === 0 && (
        <div className="absolute inset-0 flex flex-col items-center justify-center gap-4 p-8 text-center">
          <Network size={36} strokeWidth={1.25} className="text-[var(--color-paper-300)]" />
          <div>
            <p className="text-[14px] font-semibold text-[var(--color-ink-800)]">No concept graph yet</p>
            <p className="text-[12px] text-[var(--color-paper-500)] mt-1 max-w-sm">
              Extract a glossary first (in the Glossary tab). The map shows which articles each term touches —
              circles are articles, squares are terms.
            </p>
          </div>
          {error && <p className="text-[12px] text-[var(--color-error-600)]">{error}</p>}
          <button
            onClick={load}
            className="flex items-center gap-1.5 px-4 py-2 rounded-[var(--radius-sm)] bg-[var(--color-ink-800)] text-white text-[12px] font-semibold hover:bg-[var(--color-ink-700)] transition-paper"
          >
            <RefreshCw size={13} strokeWidth={2} /> Reload
          </button>
        </div>
      )}

      {graph.nodes.length > 0 && (
        <>
          <ForceGraph2D
            ref={fgRef}
            graphData={graph}
            width={size.width}
            height={size.height}
            backgroundColor="rgba(0,0,0,0)"
            nodeRelSize={6}
            linkColor={() => "rgba(100, 116, 139, 0.25)"}
            linkWidth={(l) => {
              const s = typeof l.source === "string" ? l.source : (l.source as GraphNode).id;
              const t = typeof l.target === "string" ? l.target : (l.target as GraphNode).id;
              return highlighted.size > 0 && (highlighted.has(s) && highlighted.has(t)) ? 1.5 : 0.6;
            }}
            nodeCanvasObject={(node, ctx, scale) =>
              paintNode(node as GraphNode, ctx, scale, highlighted, hovered, selected)
            }
            nodePointerAreaPaint={(node, color, ctx) => {
              const n = node as GraphNode;
              const size = (n.kind === "term" ? 6 : 4) + Math.min(8, Math.sqrt(n.degree) * 1.4) + 2;
              ctx.fillStyle = color;
              ctx.beginPath();
              ctx.arc(n.x ?? 0, n.y ?? 0, size, 0, Math.PI * 2);
              ctx.fill();
            }}
            onNodeHover={(node) => setHovered(node ? (node as GraphNode).id : null)}
            onNodeClick={(node) => {
              const n = node as GraphNode;
              if (n.kind === "term") {
                setSelected((cur) => (cur === n.id ? null : n.id));
              } else if (n.pdfPath) {
                api.openPdfPath(n.pdfPath).catch(() => {});
              }
            }}
            onBackgroundClick={() => setSelected(null)}
            cooldownTicks={120}
            d3VelocityDecay={0.25}
          />
          <div className="absolute top-3 left-3 z-10 flex items-center gap-3 px-3 py-2 rounded-[var(--radius-sm)] bg-white/90 backdrop-blur border border-[var(--color-paper-200)] text-[11px] text-[var(--color-paper-600)] shadow-sm">
            <span className="flex items-center gap-1.5">
              <span className="w-3 h-3 rounded-[2px] bg-[rgba(94,234,212,0.92)] border border-[rgba(15,118,110,0.9)]" />
              {counts.articles} article{counts.articles === 1 ? "" : "s"}
            </span>
            <span className="flex items-center gap-1.5">
              <span className="w-3 h-3 rounded-full bg-[rgba(167,139,250,0.85)] border border-[rgba(107,70,193,0.9)]" />
              {counts.terms} term{counts.terms === 1 ? "" : "s"}
            </span>
            <span className="text-[var(--color-paper-400)]">·</span>
            <span>{counts.links} link{counts.links === 1 ? "" : "s"}</span>
          </div>
          <button
            onClick={load}
            className="absolute top-3 right-3 z-10 flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] bg-white border border-[var(--color-paper-300)] text-[11px] font-medium text-[var(--color-paper-600)] hover:text-[var(--color-ink-800)] shadow-sm transition-paper"
          >
            <RefreshCw size={11} strokeWidth={2} /> Reload
          </button>
          {hovered && hovered !== selected && (() => {
            const n = graph.nodes.find((x) => x.id === hovered);
            if (!n) return null;
            return (
              <div className="absolute bottom-3 left-1/2 -translate-x-1/2 z-10 max-w-[60%] px-3 py-2 rounded-[var(--radius-sm)] bg-white border border-[var(--color-paper-300)] shadow-md text-[12px]">
                <div className="flex items-center gap-2">
                  <span className={
                    "w-2.5 h-2.5 inline-block " +
                    (n.kind === "article" ? "rounded-[2px] bg-[rgba(94,234,212,0.92)]" : "rounded-full bg-[rgba(167,139,250,0.85)]")
                  } />
                  {n.kind === "article" && (
                    <span className="font-mono text-[10px] px-1.5 py-0.5 rounded-[var(--radius-xs)] bg-[var(--color-paper-100)] text-[var(--color-paper-600)]">
                      #{n.articleId}
                    </span>
                  )}
                  <span className="font-semibold text-[var(--color-ink-800)] truncate">{n.label}</span>
                </div>
                <div className="text-[11px] text-[var(--color-paper-500)] mt-0.5">
                  {n.kind === "term"
                    ? `${n.degree} article${n.degree === 1 ? "" : "s"} reference this term · click for definition`
                    : [n.authors, n.year].filter(Boolean).join(" · ") + ` · ${n.degree} term${n.degree === 1 ? "" : "s"}`}
                  {n.kind === "article" && n.pdfPath && (
                    <span className="ml-2 italic text-[var(--color-accent-700)]">click to open PDF</span>
                  )}
                </div>
              </div>
            );
          })()}

          {/* Selected-term definition card */}
          {selected && (() => {
            const n = graph.nodes.find((x) => x.id === selected);
            if (!n || n.kind !== "term") return null;
            return (
              <div className="absolute bottom-3 left-1/2 -translate-x-1/2 z-10 w-[70%] max-w-2xl px-4 py-3 rounded-[var(--radius-sm)] bg-white border border-[var(--color-accent-300)] shadow-lg text-[12px]">
                <div className="flex items-start justify-between gap-3">
                  <div className="flex items-center gap-2 flex-wrap min-w-0">
                    <span className="w-2.5 h-2.5 rounded-full bg-[rgba(167,139,250,0.85)] border border-[rgba(107,70,193,0.9)] shrink-0" />
                    <span className="font-semibold text-[13px] text-[var(--color-ink-900)]">{n.label}</span>
                    {n.aliases && n.aliases.length > 0 && (
                      <span className="text-[11px] text-[var(--color-paper-500)] italic">
                        also known as {n.aliases.join(", ")}
                      </span>
                    )}
                    <span className="text-[10px] font-medium px-1.5 py-0.5 rounded-full bg-[var(--color-paper-100)] text-[var(--color-paper-500)]">
                      {n.degree} article{n.degree === 1 ? "" : "s"}
                    </span>
                  </div>
                  <button
                    type="button"
                    onClick={() => setSelected(null)}
                    className="text-[var(--color-paper-400)] hover:text-[var(--color-ink-800)] shrink-0 -mt-1"
                    aria-label="Close definition"
                  >
                    ×
                  </button>
                </div>
                <p className="text-[12px] text-[var(--color-paper-700)] leading-relaxed mt-1.5">
                  {n.definition || <span className="italic text-[var(--color-paper-400)]">No definition recorded.</span>}
                </p>
              </div>
            );
          })()}
          {error && (
            <div className="absolute bottom-3 right-3 z-10 px-3 py-2 rounded-[var(--radius-sm)] bg-white border border-[var(--color-error-200)] text-[12px] text-[var(--color-error-600)] shadow-sm">
              {error}
            </div>
          )}
        </>
      )}
    </div>
  );
}
