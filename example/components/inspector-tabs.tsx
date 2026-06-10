"use client";

/** Tab host for the three Phalcon-style panels. */

import { useState } from "react";
import type { TraceReport } from "@/lib/types";
import { BalanceChanges } from "./balance-changes";
import { FundFlowGraph } from "./fund-flow";
import { InvocationFlow } from "./invocation-flow";

const TABS = ["Invocation Flow", "Balance Changes", "Fund Flow"] as const;
type Tab = (typeof TABS)[number];

function countFrames(report: TraceReport): number {
  let n = 0;
  const walk = (f: NonNullable<TraceReport["trace"]>) => {
    n++;
    for (const c of f.children ?? []) walk(c);
  };
  if (report.trace) walk(report.trace);
  return n;
}

export function InspectorTabs({ report }: { report: TraceReport }) {
  const [tab, setTab] = useState<Tab>("Invocation Flow");
  const counts: Record<Tab, number> = {
    "Invocation Flow": countFrames(report),
    "Balance Changes": report.balanceChanges?.changes.length ?? 0,
    "Fund Flow": report.fundFlow?.edges.length ?? 0,
  };

  return (
    <section className="rise mt-4 rounded-lg border border-hairline bg-panel" style={{ animationDelay: "80ms" }}>
      <div className="flex items-center gap-1 border-b border-hairline px-2">
        {TABS.map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setTab(t)}
            className={`relative cursor-pointer px-3.5 py-2.5 text-[13px] transition-colors ${
              tab === t ? "text-ink" : "text-dim hover:text-ink"
            }`}
          >
            {t}
            <span className="ml-1.5 font-mono text-[10.5px] text-faint">{counts[t]}</span>
            {tab === t && (
              <span className="absolute inset-x-3 -bottom-px h-px bg-accent" />
            )}
          </button>
        ))}
      </div>
      {tab === "Invocation Flow" && <InvocationFlow report={report} />}
      {tab === "Balance Changes" && <BalanceChanges report={report} />}
      {tab === "Fund Flow" && <FundFlowGraph report={report} />}
    </section>
  );
}
