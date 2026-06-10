"use client";

/**
 * Phalcon-style fund-flow graph: left-to-right layered layout (dagre) on a
 * dotted canvas, one curved edge per transfer with `order amount SYMBOL`
 * labels, color-keyed by asset, keyboard-navigable (← → step through
 * transfers in execution order, Esc clears).
 *
 * Consumes `report.fundFlow` JSON directly — no Mermaid anywhere.
 */

import dagre from "@dagrejs/dagre";
import {
  BaseEdge,
  Background,
  BackgroundVariant,
  Controls,
  EdgeLabelRenderer,
  Handle,
  MarkerType,
  Position,
  ReactFlow,
  type Edge,
  type EdgeProps,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useEffect, useMemo, useState } from "react";
import {
  amountText,
  amountTextCompact,
  assetColor,
  assetSymbol,
  txKindBadge,
} from "@/lib/format";
import type { FlowNodeKind, TraceReport } from "@/lib/types";
import { Chip } from "./ui";

const NODE_W = 232;
const NODE_H = 56;

type AddrNodeData = {
  label?: string;
  address: string;
  kind: FlowNodeKind;
  role?: "sender" | "receiver";
};
type AddrNode = Node<AddrNodeData, "addr">;

type XY = { x: number; y: number };

type TransferEdgeData = {
  order: number;
  amount: string;
  full: string;
  symbol: string;
  color: string;
  /** Dagre-routed waypoints (flow coordinates) between the two handles. */
  waypoints: XY[];
  /** Dagre-reserved label anchor — guaranteed clear of nodes/other labels. */
  labelX: number;
  labelY: number;
  dimmed: boolean;
  active: boolean;
};
type TransferEdge = Edge<TransferEdgeData, "transfer">;

function midAddr(a: string): string {
  return a.length > 24 ? `${a.slice(0, 12)}…${a.slice(-8)}` : a;
}

/** Approximate rendered size of an edge label chip (11px mono). */
function labelSize(text: string): { width: number; height: number } {
  return { width: Math.ceil(text.length * 6.7) + 14, height: 22 };
}

/**
 * Smooth (Catmull-Rom) path through the handle endpoints and dagre's routed
 * waypoints. Dagre threads edges through inter-rank channels around nodes
 * and label boxes — following its polyline is what keeps long edges from
 * cutting straight through intermediate nodes.
 */
function smoothPath(pts: XY[]): string {
  if (pts.length < 2) return "";
  if (pts.length === 2) {
    const [a, b] = pts;
    const dx = (b.x - a.x) * 0.4;
    return `M ${a.x},${a.y} C ${a.x + dx},${a.y} ${b.x - dx},${b.y} ${b.x},${b.y}`;
  }
  let d = `M ${pts[0].x},${pts[0].y}`;
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[Math.max(0, i - 1)];
    const p1 = pts[i];
    const p2 = pts[i + 1];
    const p3 = pts[Math.min(pts.length - 1, i + 2)];
    const c1x = p1.x + (p2.x - p0.x) / 6;
    const c1y = p1.y + (p2.y - p0.y) / 6;
    const c2x = p2.x - (p3.x - p1.x) / 6;
    const c2y = p2.y - (p3.y - p1.y) / 6;
    d += ` C ${c1x},${c1y} ${c2x},${c2y} ${p2.x},${p2.y}`;
  }
  return d;
}

const KIND_GLYPH: Record<FlowNodeKind, { glyph: string; cls: string }> = {
  token: { glyph: "◈", cls: "text-accent" },
  contract: { glyph: "▤", cls: "text-dim" },
  eoa: { glyph: "◉", cls: "text-warn" },
  account: { glyph: "○", cls: "text-faint" },
};

function AddrNodeView({ data }: NodeProps<AddrNode>) {
  const g = KIND_GLYPH[data.kind];
  const ring =
    data.role === "sender"
      ? "border-warn/50"
      : data.role === "receiver"
        ? "border-accent/50"
        : "border-hairline-2";
  // Invisible handles on both sides; each edge picks the pair of sides that
  // face each other, so return flows don't wrap around the nodes.
  const handleCls = "!pointer-events-none !opacity-0";
  return (
    <div
      className={`w-[232px] rounded-md border ${ring} bg-panel px-3 py-2 shadow-[0_2px_10px_rgba(0,0,0,0.35)]`}
    >
      <Handle id="t-left" type="target" position={Position.Left} className={handleCls} />
      <Handle id="s-left" type="source" position={Position.Left} className={handleCls} />
      <Handle id="t-right" type="target" position={Position.Right} className={handleCls} />
      <Handle id="s-right" type="source" position={Position.Right} className={handleCls} />
      <div className="flex items-center gap-2">
        <span className={`text-[13px] ${g.cls}`}>{g.glyph}</span>
        <span className="truncate text-[12.5px] text-ink">
          {data.label ?? midAddr(data.address)}
        </span>
      </div>
      {data.label && (
        <div className="mt-0.5 truncate pl-[21px] font-mono text-[10.5px] text-faint">
          {midAddr(data.address)}
        </div>
      )}
    </div>
  );
}

function TransferEdgeView({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  data,
  markerEnd,
}: EdgeProps<TransferEdge>) {
  // Endpoints come from the chosen handles; interior waypoints from dagre.
  const pts: XY[] = [
    { x: sourceX, y: sourceY },
    ...(data?.waypoints ?? []),
    { x: targetX, y: targetY },
  ];
  const path = smoothPath(pts);
  const labelX = data?.labelX ?? (sourceX + targetX) / 2;
  const labelY = data?.labelY ?? (sourceY + targetY) / 2;
  const opacity = data?.dimmed ? 0.18 : 1;
  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerEnd={markerEnd}
        style={{
          stroke: data?.color,
          strokeWidth: data?.active ? 2.4 : 1.4,
          opacity,
          transition: "opacity 120ms, stroke-width 120ms",
        }}
      />
      <EdgeLabelRenderer>
        <div
          style={{
            transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
            opacity,
          }}
          className={`pointer-events-none absolute ${data?.active ? "z-20" : ""}`}
          title={`${(data?.order ?? 0) + 1}: ${data?.full} ${data?.symbol}`}
        >
          <span
            className="rounded-[3px] border px-1 py-px font-mono text-[11px] leading-4 whitespace-nowrap"
            style={{
              background: "color-mix(in srgb, var(--bg) 88%, transparent)",
              borderColor: data?.active
                ? data.color
                : "color-mix(in srgb, var(--hairline) 90%, transparent)",
            }}
          >
            <span className="text-faint">{(data?.order ?? 0) + 1} </span>
            <span className="text-ink">{data?.amount}</span>
            <span style={{ color: data?.color }}> {data?.symbol}</span>
          </span>
        </div>
      </EdgeLabelRenderer>
    </>
  );
}

const nodeTypes = { addr: AddrNodeView };
const edgeTypes = { transfer: TransferEdgeView };

export function FundFlowGraph({ report }: { report: TraceReport }) {
  const flow = report.fundFlow;
  const [active, setActive] = useState<number | null>(null);

  const edgeCount = flow?.edges.length ?? 0;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (edgeCount === 0) return;
      if (e.key === "Escape") setActive(null);
      else if (e.key === "ArrowRight")
        setActive((a) => (a === null ? 0 : Math.min(a + 1, edgeCount - 1)));
      else if (e.key === "ArrowLeft")
        setActive((a) => (a === null ? edgeCount - 1 : Math.max(a - 1, 0)));
      else return;
      e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [edgeCount]);

  const { nodes, edges, ordered } = useMemo(() => {
    if (!flow || flow.edges.length === 0) {
      return { nodes: [] as AddrNode[], edges: [] as TransferEdge[], ordered: [] };
    }
    const ordered = [...flow.edges].sort((a, b) => a.order - b.order);

    // Let dagre lay out EDGES and LABELS, not just nodes: as a multigraph,
    // every transfer gets its own routed polyline through the inter-rank
    // channels (around nodes), and giving each edge label real dimensions
    // reserves canvas space for it — no more labels under nodes or stacked
    // on one another. "greedy" cycle-breaking handles the round-trips that
    // dominate swap flows.
    const g = new dagre.graphlib.Graph({ multigraph: true });
    g.setDefaultEdgeLabel(() => ({}));
    g.setGraph({
      rankdir: "LR",
      acyclicer: "greedy",
      nodesep: 44,
      ranksep: 110,
      edgesep: 26,
      marginx: 40,
      marginy: 40,
    });
    for (const n of flow.nodes) g.setNode(n.id, { width: NODE_W, height: NODE_H });

    const displayOf = (e: (typeof ordered)[number]) => ({
      amount: amountTextCompact(e.amount),
      symbol: assetSymbol(report, e.asset),
    });
    for (const e of ordered) {
      const d = displayOf(e);
      const text = `${e.order + 1} ${d.amount} ${d.symbol}`;
      g.setEdge(e.from, e.to, { ...labelSize(text), labelpos: "c" }, `t${e.id}`);
    }
    dagre.layout(g);

    const nodes: AddrNode[] = flow.nodes.map((n) => {
      const pos = g.node(n.id);
      return {
        id: n.id,
        type: "addr",
        position: { x: pos.x - NODE_W / 2, y: pos.y - NODE_H / 2 },
        data: {
          label: n.label,
          address: n.id,
          kind: n.kind,
          role: txKindBadge(report, n.id),
        },
        sourcePosition: Position.Right,
        targetPosition: Position.Left,
      };
    });

    const edges: TransferEdge[] = ordered.map((e, i) => {
      const color = assetColor(e.asset);
      const d = displayOf(e);
      const routed = g.edge(e.from, e.to, `t${e.id}`) as
        | { points?: XY[]; x?: number; y?: number }
        | undefined;
      // Drop dagre's border endpoints — React Flow supplies exact handle
      // coordinates — and keep the interior channel waypoints.
      const waypoints = (routed?.points ?? []).slice(1, -1);
      const mid = waypoints[Math.floor(waypoints.length / 2)];
      const labelX = routed?.x ?? mid?.x ?? 0;
      const labelY = routed?.y ?? mid?.y ?? 0;
      // Connect the sides that face each other: a target left of its source
      // is reached source-left → target-right instead of wrapping around.
      const backward = g.node(e.to).x < g.node(e.from).x;
      return {
        id: `t${e.id}`,
        type: "transfer",
        source: e.from,
        target: e.to,
        sourceHandle: backward ? "s-left" : "s-right",
        targetHandle: backward ? "t-right" : "t-left",
        interactionWidth: 20,
        markerEnd: { type: MarkerType.ArrowClosed, color, width: 14, height: 14 },
        data: {
          order: e.order,
          amount: d.amount,
          full: amountText(e.amount),
          symbol: d.symbol,
          color,
          waypoints,
          labelX,
          labelY,
          dimmed: active !== null && i !== active,
          active: i === active,
        },
      };
    });
    return { nodes, edges, ordered };
  }, [flow, report, active]);

  if (!flow || flow.edges.length === 0) {
    return <div className="p-6 text-sm text-dim">No asset transfers in this transaction.</div>;
  }

  const sel = active !== null ? ordered[active] : null;

  return (
    <div className="relative h-[640px]">
      <ReactFlow
        colorMode="dark"
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onEdgeClick={(_, edge) => setActive(edges.findIndex((e) => e.id === edge.id))}
        onPaneClick={() => setActive(null)}
        fitView
        fitViewOptions={{ padding: 0.16, maxZoom: 1.15 }}
        minZoom={0.15}
        nodesDraggable
        nodesConnectable={false}
        edgesFocusable={false}
        proOptions={{ hideAttribution: false }}
      >
        <Background variant={BackgroundVariant.Dots} gap={22} size={1.2} color="#202a35" />
        <Controls position="bottom-right" showInteractive={false} />
      </ReactFlow>

      <div className="pointer-events-none absolute top-3 left-3 flex items-center gap-1.5 rounded-md border border-hairline-2 bg-panel/90 px-2.5 py-1.5 text-[11px] text-dim">
        <kbd className="rounded border border-hairline-2 bg-panel-2 px-1 font-mono text-[10px]">
          Esc
        </kbd>
        <kbd className="rounded border border-hairline-2 bg-panel-2 px-1 font-mono text-[10px]">
          ←
        </kbd>
        <kbd className="rounded border border-hairline-2 bg-panel-2 px-1 font-mono text-[10px]">
          →
        </kbd>
        Use arrow keys to navigate
      </div>

      {sel && (
        <div className="absolute bottom-3 left-3 flex items-center gap-2.5 rounded-md border border-hairline-2 bg-panel/95 px-3 py-2 font-mono text-[12px]">
          <Chip tone="accent">#{sel.order + 1}</Chip>
          <span className="text-dim">{midAddr(sel.from)}</span>
          <span className="text-faint">→</span>
          <span className="text-dim">{midAddr(sel.to)}</span>
          <span className="text-ink">
            {amountText(sel.amount)} {assetSymbol(report, sel.asset)}
          </span>
          <span className="text-faint">via {sel.origin.kind}</span>
        </div>
      )}
    </div>
  );
}
