"use client";

/**
 * Phalcon-style invocation flow: a line-numbered, expandable call tree with
 * events interleaved at their exact positions, kind chips, decoded
 * signatures, value/gas annotations, storage accesses (deep mode), search,
 * and visibility toggles.
 */

import { useMemo, useState } from "react";
import { labelFor, shortHex, txKindBadge, weiToEth } from "@/lib/format";
import type {
  CallKindJson,
  FrameJson,
  FrameLogJson,
  TraceReport,
} from "@/lib/types";
import { AddressTag, Chip, Toggle } from "./ui";

type Row =
  | {
      t: "call";
      n: number;
      depth: number;
      frame: FrameJson;
      ancestors: number[];
      hasChildren: boolean;
      inStatic: boolean;
    }
  | {
      t: "event";
      n: number;
      depth: number;
      log: FrameLogJson;
      host: FrameJson;
      ancestors: number[];
      inStatic: boolean;
    }
  | {
      t: "sstore" | "sload";
      depth: number;
      slot: string;
      value?: string;
      previous?: string;
      host: FrameJson;
      ancestors: number[];
      inStatic: boolean;
    };

function buildRows(root: FrameJson): { rows: Row[]; maxDepth: number } {
  const rows: Row[] = [];
  let n = 0;
  let maxDepth = 0;

  const walk = (f: FrameJson, depth: number, ancestors: number[], inStatic: boolean) => {
    maxDepth = Math.max(maxDepth, depth);
    const children = f.children ?? [];
    const logs = f.logs ?? [];
    rows.push({
      t: "call",
      n: n++,
      depth,
      frame: f,
      ancestors,
      hasChildren: children.length + logs.length > 0,
      inStatic,
    });
    const childAncestors = [...ancestors, f.id];
    const childStatic = inStatic || f.kind === "STATICCALL";
    let li = 0;
    const flush = (upto: number | null) => {
      while (li < logs.length) {
        const p = logs[li].position;
        const due = upto === null ? true : p !== undefined && p <= upto;
        if (!due) break;
        rows.push({
          t: "event",
          n: n++,
          depth: depth + 1,
          log: logs[li],
          host: f,
          ancestors: childAncestors,
          inStatic: childStatic,
        });
        li++;
      }
    };
    children.forEach((c, ci) => {
      flush(ci);
      walk(c, depth + 1, childAncestors, childStatic);
    });
    flush(null);
    for (const r of f.storageReads ?? []) {
      rows.push({
        t: "sload",
        depth: depth + 1,
        slot: r.slot,
        value: r.value,
        host: f,
        ancestors: childAncestors,
        inStatic: childStatic,
      });
    }
    for (const w of f.storageWrites ?? []) {
      rows.push({
        t: "sstore",
        depth: depth + 1,
        slot: w.slot,
        value: w.value,
        previous: w.previous,
        host: f,
        ancestors: childAncestors,
        inStatic: childStatic,
      });
    }
  };
  walk(root, 0, [], false);
  return { rows, maxDepth };
}

const KIND_TONE: Record<CallKindJson, "accent" | "cyan" | "violet" | "pos" | "neg"> = {
  CALL: "accent",
  STATICCALL: "cyan",
  DELEGATECALL: "violet",
  CALLCODE: "violet",
  CREATE: "pos",
  CREATE2: "pos",
  SELFDESTRUCT: "neg",
};

function outputText(f: FrameJson): string | undefined {
  if (f.kind === "CREATE" || f.kind === "CREATE2") return undefined;
  const out = f.output;
  if (!out || out === "0x") return undefined;
  if (out.length === 66) {
    const body = out.slice(2);
    if (/^0+1$/.test(body)) return "true";
    if (/^0+$/.test(body)) return "false";
  }
  return shortHex(out, 20);
}

function rowHaystack(report: TraceReport, row: Row): string {
  if (row.t === "call") {
    const f = row.frame;
    return [
      f.from,
      f.to,
      labelFor(report, f.to ?? ""),
      f.decoded?.name,
      f.decoded?.selector,
      ...(f.decoded?.params?.map((p) => p.value) ?? []),
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
  }
  if (row.t === "event") {
    return [
      row.log.address,
      labelFor(report, row.log.address),
      row.log.decoded?.name,
      ...(row.log.decoded?.params?.map((p) => p.value) ?? []),
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
  }
  return `${row.slot} ${row.value ?? ""}`.toLowerCase();
}

const RENDER_CAP = 2500;

export function InvocationFlow({ report }: { report: TraceReport }) {
  const root = report.trace;
  const built = useMemo(() => (root ? buildRows(root) : { rows: [], maxDepth: 0 }), [root]);
  const { rows } = built;

  const [collapsed, setCollapsed] = useState<Set<number>>(new Set());
  const [expandLevel, setExpandLevel] = useState<string>("all");
  const [showStatic, setShowStatic] = useState(true);
  const [showGas, setShowGas] = useState(false);
  const [showSstore, setShowSstore] = useState(false);
  const [showSload, setShowSload] = useState(false);
  const [query, setQuery] = useState("");

  const applyExpandLevel = (level: string) => {
    setExpandLevel(level);
    if (level === "all") {
      setCollapsed(new Set());
      return;
    }
    const depth = parseInt(level, 10);
    const next = new Set<number>();
    for (const r of rows) {
      if (r.t === "call" && r.hasChildren && r.depth >= depth) next.add(r.frame.id);
    }
    setCollapsed(next);
  };

  const toggleFrame = (id: number) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const q = query.trim().toLowerCase();
  const visible = useMemo(() => {
    let out: Row[];
    if (q) {
      const keep = new Set<number>(); // row indices
      rows.forEach((row, i) => {
        if (rowHaystack(report, row).includes(q)) {
          keep.add(i);
          // reveal ancestor call rows for context
          for (const a of row.ancestors) {
            const ai = rows.findIndex((r) => r.t === "call" && r.frame.id === a);
            if (ai >= 0) keep.add(ai);
          }
        }
      });
      out = rows.filter((_, i) => keep.has(i));
    } else {
      out = rows.filter((row) => row.ancestors.every((a) => !collapsed.has(a)));
    }
    return out.filter((row) => {
      if (!showStatic && (row.inStatic || (row.t === "call" && row.frame.kind === "STATICCALL")))
        return false;
      if (row.t === "sstore" && !showSstore) return false;
      if (row.t === "sload" && !showSload) return false;
      return true;
    });
  }, [rows, report, q, collapsed, showStatic, showSstore, showSload]);

  if (!root) {
    return <div className="p-6 text-dim text-sm">No trace available for this transaction.</div>;
  }

  const hasStorageData = rows.some((r) => r.t === "sstore" || r.t === "sload");

  return (
    <div>
      <div className="flex flex-wrap items-center gap-x-5 gap-y-2 border-b border-hairline px-4 py-2.5">
        <label className="flex items-center gap-1.5 text-[12px] text-dim">
          Expand
          <select
            value={expandLevel}
            onChange={(e) => applyExpandLevel(e.target.value)}
            className="cursor-pointer rounded border border-hairline-2 bg-panel-2 px-1.5 py-0.5 font-mono text-[11px] text-ink outline-none"
          >
            <option value="all">All</option>
            <option value="1">1</option>
            <option value="2">2</option>
            <option value="3">3</option>
            <option value="5">5</option>
          </select>
        </label>
        <Toggle on={showStatic} onChange={setShowStatic} label="Static Call" />
        <Toggle on={showGas} onChange={setShowGas} label="Gas Used" />
        <Toggle on={showSstore} onChange={setShowSstore} label="SStore" />
        <Toggle on={showSload} onChange={setShowSload} label="SLoad" />
        {(showSstore || showSload) && !hasStorageData && (
          <Chip tone="warn">storage data needs --deep</Chip>
        )}
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search by contract label / address / function…"
          className="ml-auto w-72 max-w-full rounded border border-hairline-2 bg-panel-2 px-2.5 py-1 font-mono text-[12px] text-ink placeholder:text-faint outline-none focus:border-accent/50"
        />
      </div>

      <div className="overflow-x-auto py-1.5 font-mono text-[12.5px] leading-[1.9]">
        {visible.slice(0, RENDER_CAP).map((row, i) => (
          <RowLine
            key={i}
            report={report}
            row={row}
            collapsed={row.t === "call" ? collapsed.has(row.frame.id) : false}
            onToggle={row.t === "call" ? () => toggleFrame(row.frame.id) : undefined}
            showGas={showGas}
          />
        ))}
        {visible.length > RENDER_CAP && (
          <div className="px-4 py-2 text-dim">
            … {visible.length - RENDER_CAP} more rows (use search to narrow)
          </div>
        )}
        {visible.length === 0 && <div className="px-4 py-3 text-dim">no rows match</div>}
      </div>
    </div>
  );
}

function Guides({ depth }: { depth: number }) {
  return (
    <span className="flex shrink-0 self-stretch">
      {Array.from({ length: depth }, (_, i) => (
        <span key={i} className="ml-[7px] block w-[9px] border-l border-hairline" />
      ))}
    </span>
  );
}

function RowLine({
  report,
  row,
  collapsed,
  onToggle,
  showGas,
}: {
  report: TraceReport;
  row: Row;
  collapsed: boolean;
  onToggle?: () => void;
  showGas: boolean;
}) {
  const num = row.t === "call" || row.t === "event" ? row.n : null;
  return (
    <div className="group flex items-start gap-0 px-2 hover:bg-panel-2/60">
      <span className="w-9 shrink-0 select-none pr-2 text-right text-[11px] leading-[1.9] text-faint">
        {num}
      </span>
      <Guides depth={row.depth} />
      {row.t === "call" && onToggle && row.hasChildren ? (
        <button
          onClick={onToggle}
          className="mr-1 w-4 shrink-0 cursor-pointer text-faint hover:text-ink"
          aria-label={collapsed ? "expand" : "collapse"}
        >
          {collapsed ? "⊞" : "⊟"}
        </button>
      ) : (
        <span className="mr-1 w-4 shrink-0" />
      )}
      {row.t === "call" && <CallContent report={report} f={row.frame} showGas={showGas} />}
      {row.t === "event" && <EventContent report={report} log={row.log} />}
      {(row.t === "sstore" || row.t === "sload") && (
        <span className="flex min-w-0 flex-wrap items-center gap-x-1.5">
          <Chip tone={row.t === "sstore" ? "warn" : "dim"}>
            {row.t === "sstore" ? "SSTORE" : "SLOAD"}
          </Chip>
          <span className="text-dim">{shortHex(row.slot, 16)}</span>
          {row.t === "sload" ? (
            <span className="text-ink">→ {shortHex(row.value ?? "0x?", 16)}</span>
          ) : (
            <>
              <span className="text-ink">← {shortHex(row.value ?? "0x", 16)}</span>
              {row.previous && (
                <span className="text-faint">(was {shortHex(row.previous, 14)})</span>
              )}
            </>
          )}
        </span>
      )}
    </div>
  );
}

function CallContent({
  report,
  f,
  showGas,
}: {
  report: TraceReport;
  f: FrameJson;
  showGas: boolean;
}) {
  const isCreate = f.kind === "CREATE" || f.kind === "CREATE2";
  const target = isCreate || f.kind !== "SELFDESTRUCT" ? f.to : f.to;
  const value = BigInt(f.value ?? "0x0");
  const out = outputText(f);
  return (
    <span className="flex min-w-0 flex-wrap items-center gap-x-1.5">
      <Chip tone={KIND_TONE[f.kind]}>{f.kind}</Chip>
      {f.kind === "SELFDESTRUCT" && (
        <>
          <AddressTag address={f.from} label={labelFor(report, f.from)} />
          <span className="text-faint">→</span>
        </>
      )}
      {target ? (
        <AddressTag
          address={target}
          label={labelFor(report, target)}
          badge={txKindBadge(report, target)}
        />
      ) : (
        <span className="text-faint">?</span>
      )}
      {!isCreate && f.decoded?.name && (
        <span className="min-w-0">
          <span className="text-faint">.</span>
          <span className="text-accent">{f.decoded.name}</span>
          <span className="text-faint">(</span>
          <span className="text-dim">
            {(f.decoded.params ?? []).map((p, i) => (
              <span key={i} title={p.value}>
                {i > 0 && ", "}
                {p.name}={shortHex(p.value, 24)}
              </span>
            ))}
          </span>
          <span className="text-faint">)</span>
        </span>
      )}
      {!isCreate && !f.decoded?.name && f.decoded?.selector && f.input !== "0x" && (
        <Chip tone="dim" title={f.input}>
          {f.decoded.selector}
        </Chip>
      )}
      {isCreate && f.input && f.input !== "0x" && (
        <Chip tone="dim" title="init code">
          {(f.input.length - 2) / 2} bytes
        </Chip>
      )}
      {value > 0n && (
        <span className="text-pos">
          {"{"}
          {weiToEth(f.value)} {report.nativeSymbol}
          {"}"}
        </span>
      )}
      {showGas && f.gasUsed > 0 && <span className="text-faint">gas={f.gasUsed}</span>}
      {out && (
        <span className="text-faint">
          ▸ <span className="text-dim">({out})</span>
        </span>
      )}
      {f.error && (
        <span className="text-neg">
          ✗ {f.error}
          {f.revertReason ? `: ${f.revertReason}` : ""}
        </span>
      )}
    </span>
  );
}

function EventContent({ report, log }: { report: TraceReport; log: FrameLogJson }) {
  return (
    <span className="flex min-w-0 flex-wrap items-center gap-x-1.5">
      <Chip tone="pos">EVENT</Chip>
      <AddressTag address={log.address} label={labelFor(report, log.address)} />
      {log.decoded ? (
        <span className="min-w-0">
          <span className="text-faint">.</span>
          <span className="text-pos">{log.decoded.name}</span>
          <span className="text-faint">(</span>
          <span className="text-dim">
            {(log.decoded.params ?? []).map((p, i) => (
              <span key={i} title={p.value}>
                {i > 0 && ", "}
                {p.name}={shortHex(p.value, 24)}
              </span>
            ))}
          </span>
          <span className="text-faint">)</span>
        </span>
      ) : (
        <Chip tone="dim" title={log.topics[0] ?? ""}>
          {log.topics.length > 0 ? shortHex(log.topics[0], 14) : "anonymous"}
        </Chip>
      )}
      {log.logIndex !== undefined && <span className="text-faint">#{log.logIndex}</span>}
    </span>
  );
}
