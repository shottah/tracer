"use client";

/** Tenderly/Phalcon-style balance changes: per-account asset deltas. */

import {
  amountText,
  assetColor,
  assetSymbol,
  shortAddr,
  signedAmountText,
  txKindBadge,
} from "@/lib/format";
import type { TraceReport } from "@/lib/types";
import { AddressTag, Chip } from "./ui";

export function BalanceChanges({ report }: { report: TraceReport }) {
  const bc = report.balanceChanges;
  if (!bc || bc.changes.length === 0) {
    return <div className="p-6 text-sm text-dim">No balance changes.</div>;
  }
  return (
    <div>
      <div className="flex items-center gap-3 border-b border-hairline px-4 py-2.5 text-[12px] text-dim">
        <Chip tone={bc.nativeSource === "prestate" ? "pos" : "warn"}>
          native: {bc.nativeSource === "prestate" ? "prestate (exact)" : "derived"}
        </Chip>
        <Chip tone="dim">gas {bc.gasIncluded ? "included" : "excluded"}</Chip>
        <span className="ml-auto">{bc.changes.length} accounts</span>
      </div>

      <div className="divide-y divide-hairline">
        {bc.changes.map((change) => (
          <div key={change.address} className="px-4 py-2.5">
            <div className="mb-1 flex items-center text-[13px]">
              <AddressTag
                address={change.address}
                label={change.label}
                badge={txKindBadge(report, change.address)}
              />
              <span className="ml-2 font-mono text-[11px] text-faint">
                {shortAddr(change.address)}
              </span>
            </div>
            <div className="flex flex-col gap-0.5">
              {change.native && (
                <AssetRow
                  dot="#34d399"
                  name={report.nativeSymbol}
                  sub={
                    change.native.gasFee
                      ? `incl. gas ${amountText(change.native.gasFee)} ${report.nativeSymbol}`
                      : undefined
                  }
                  delta={signedAmountText(change.native.delta)}
                  negative={change.native.delta.negative}
                />
              )}
              {(change.tokens ?? []).map((tc, i) => (
                <AssetRow
                  key={i}
                  dot={assetColor(tc.asset)}
                  name={assetSymbol(report, tc.asset)}
                  sub={
                    tc.asset.type !== "native" ? shortAddr(tc.asset.token) : undefined
                  }
                  count={tc.transferCount}
                  delta={signedAmountText(tc.delta)}
                  negative={tc.delta.negative}
                />
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function AssetRow({
  dot,
  name,
  sub,
  count,
  delta,
  negative,
}: {
  dot: string;
  name: string;
  sub?: string;
  count?: number;
  delta: string;
  negative: boolean;
}) {
  return (
    <div className="flex items-baseline gap-2 pl-1 font-mono text-[12.5px]">
      <span
        className="inline-block h-2 w-2 shrink-0 self-center rounded-[2px]"
        style={{ background: dot }}
      />
      <span className="text-ink">{name}</span>
      {sub && <span className="text-[11px] text-faint">{sub}</span>}
      {count !== undefined && count > 1 && (
        <span className="text-[11px] text-faint">×{count}</span>
      )}
      <span className={`ml-auto tabular-nums ${negative ? "text-neg" : "text-pos"}`}>
        {delta}
      </span>
    </div>
  );
}
