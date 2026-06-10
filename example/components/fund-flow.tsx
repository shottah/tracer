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

type TransferEdgeData = {
  order: number;
  amount: string;
  full: string;
  symbol: string;
  color: string;
  /** Vertical bow (px) at the edge midpoint — separates parallel edges. */
  offset: number;
  dimmed: boolean;
  active: boolean;
};
type TransferEdge = Edge<TransferEdgeData, "transfer">;

function midAddr(a: string): string {
  return a.length > 24 ? `${a.slice(0, 12)}…${a.slice(-8)}` : a;
}

/** Vertical gap (px) between adjacent parallel edges / their labels. */
const PARALLEL_GAP = 30;

/**
 * Cubic bezier from source to target that bows vertically by `offset` at the
 * midpoint. Unlike `getBezierPath`'s curvature (which does nothing when the
 * endpoints share a Y, as they do in a clean LR layout), this guarantees
 * parallel edges between the same node pair fan apart legibly.
 *
 * Returns `[svgPath, labelX, labelY]`; the label sits at the bow apex.
 */
function bowedPath(
  sx: number,
  sy: number,
  tx: number,
  ty: number,
  offset: number,
): [string, number, number] {
  const dx = tx - sx;
  // A cubic with both control points raised by k reaches 0.75·k at the
  // midpoint, so k = offset / 0.75 puts the apex exactly at `offset`.
  const k = offset / 0.75;
  const c1x = sx + dx * 0.33;
  const c2x = sx + dx * 0.67;
  const d = `M ${sx},${sy} C ${c1x},${sy + k} ${c2x},${ty + k} ${tx},${ty}`;
  return [d, (sx + tx) / 2, (sy + ty) / 2 + offset];
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
  const [path, labelX, labelY] = bowedPath(
    sourceX,
    sourceY,
    targetX,
    targetY,
    data?.offset ?? 0,
  );
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
    const g = new dagre.graphlib.Graph();
    g.setDefaultEdgeLabel(() => ({}));
    g.setGraph({ rankdir: "LR", nodesep: 42, ranksep: 200, marginx: 40, marginy: 40 });
    for (const n of flow.nodes) g.setNode(n.id, { width: NODE_W, height: NODE_H });
    for (const e of flow.edges) g.setEdge(e.from, e.to);
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

    const ordered = [...flow.edges].sort((a, b) => a.order - b.order);

    // Group edges that share a node pair (in either direction) so they can be
    // fanned symmetrically around the straight line between the two nodes.
    const pairKey = (e: { from: string; to: string }) =>
      [e.from, e.to].sort().join("|");
    const groupTotal = new Map<string, number>();
    for (const e of ordered) groupTotal.set(pairKey(e), (groupTotal.get(pairKey(e)) ?? 0) + 1);
    const groupSeen = new Map<string, number>();

    const edges: TransferEdge[] = ordered.map((e, i) => {
      const key = pairKey(e);
      const gi = groupSeen.get(key) ?? 0;
      groupSeen.set(key, gi + 1);
      const gn = groupTotal.get(key)!;
      // Symmetric fan: e.g. 3 edges → offsets −gap, 0, +gap.
      const offset = (gi - (gn - 1) / 2) * PARALLEL_GAP;
      const color = assetColor(e.asset);
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
          amount: amountTextCompact(e.amount),
          full: amountText(e.amount),
          symbol: assetSymbol(report, e.asset),
          color,
          offset,
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
