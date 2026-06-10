"use client";

/** Transaction summary band: status, hash, parties, gas economics, backend. */

import { useState } from "react";
import {
  gweiText,
  labelFor,
  timestampText,
  weiToEth,
} from "@/lib/format";
import type { TraceReport } from "@/lib/types";
import { AddressTag, Chip, CopyButton } from "./ui";

export function TxHeader({ report }: { report: TraceReport }) {
  const [showWarnings, setShowWarnings] = useState(false);
  const tx = report.tx;
  const fee = BigInt(tx.gasUsed) * BigInt(Math.trunc(tx.effectiveGasPrice));
  const fork = report.backend.fork;
  const warnings = report.warnings ?? [];

  return (
    <section className="rise rounded-lg border border-hairline bg-panel">
      <div className="flex flex-wrap items-center gap-3 border-b border-hairline px-5 py-3.5">
        {tx.status ? (
          <span className="rounded-md bg-pos/15 px-2 py-0.5 text-[12px] font-medium text-pos">
            ● Success
          </span>
        ) : (
          <span className="rounded-md bg-neg/15 px-2 py-0.5 text-[12px] font-medium text-neg">
            ✗ Failed
          </span>
        )}
        <span className="flex min-w-0 items-center gap-2 font-mono text-[13px] text-ink">
          <span className="truncate">{tx.hash}</span>
          <CopyButton text={tx.hash} />
        </span>
        <span className="ml-auto flex items-center gap-2">
          <Chip tone={report.backend.kind === "anvilFork" ? "warn" : "accent"}>
            {report.backend.kind === "anvilFork" ? "anvil fork" : "rpc"}
          </Chip>
          {report.backend.endpointHost && (
            <Chip tone="dim" title="endpoint (credentials redacted)">
              {report.backend.endpointHost.replace(/^https?:\/\//, "")}
            </Chip>
          )}
          <Chip tone="dim">chain {report.chainId}</Chip>
          {warnings.length > 0 && (
            <button
              type="button"
              onClick={() => setShowWarnings((v) => !v)}
              className="cursor-pointer"
            >
              <Chip tone="warn">
                {warnings.length} warning{warnings.length > 1 ? "s" : ""} {showWarnings ? "▴" : "▾"}
              </Chip>
            </button>
          )}
        </span>
      </div>

      {showWarnings && warnings.length > 0 && (
        <ul className="border-b border-hairline bg-warn/5 px-5 py-2.5 text-[12px] text-warn/90">
          {warnings.map((w, i) => (
            <li key={i} className="py-0.5">
              ⚠ {w}
            </li>
          ))}
        </ul>
      )}

      <div className="grid grid-cols-2 gap-x-8 gap-y-3 px-5 py-4 md:grid-cols-4 xl:grid-cols-6">
        <Stat label="From">
          <AddressTag address={tx.from} label={labelFor(report, tx.from)} badge="sender" />
        </Stat>
        <Stat label={tx.to ? "To" : "Created"}>
          {tx.to || tx.contractCreated ? (
            <AddressTag
              address={(tx.to ?? tx.contractCreated)!}
              label={labelFor(report, (tx.to ?? tx.contractCreated)!)}
              badge="receiver"
            />
          ) : (
            <span className="text-faint">—</span>
          )}
        </Stat>
        <Stat label="Value">
          <span className="font-mono">
            {weiToEth(tx.value)} {report.nativeSymbol}
          </span>
        </Stat>
        <Stat label="Block">
          <span className="font-mono">
            {tx.blockNumber}
            {tx.transactionIndex !== undefined && (
              <span className="text-faint"> · #{tx.transactionIndex}</span>
            )}
          </span>
        </Stat>
        <Stat label="Timestamp">
          <span className="font-mono text-[12px]">{timestampText(tx.blockTimestamp) ?? "—"}</span>
        </Stat>
        <Stat label="Nonce / Type">
          <span className="font-mono">
            {tx.nonce} <span className="text-faint">· type {tx.txType}</span>
          </span>
        </Stat>
        <Stat label="Gas used">
          <span className="font-mono">
            {tx.gasUsed.toLocaleString()}
            <span className="text-faint">
              {" "}
              / {tx.gasLimit.toLocaleString()} (
              {((tx.gasUsed / Math.max(tx.gasLimit, 1)) * 100).toFixed(1)}%)
            </span>
          </span>
        </Stat>
        <Stat label="Gas price">
          <span className="font-mono">{gweiText(tx.effectiveGasPrice)}</span>
        </Stat>
        <Stat label="Tx fee">
          <span className="font-mono">
            {weiToEth(fee.toString())} {report.nativeSymbol}
          </span>
        </Stat>
        {fork && (
          <Stat label="Fork replay">
            <span className="font-mono text-[12px]">
              @{fork.forkBlock} · {fork.replayed} replayed
              {fork.skipped > 0 ? ` · ${fork.skipped} skipped` : ""}{" "}
              {fork.fidelity &&
                (fork.fidelity.statusMatch &&
                fork.fidelity.gasUsedMatch &&
                fork.fidelity.logCountMatch ? (
                  <span className="text-pos">fidelity ✓</span>
                ) : (
                  <span className="text-neg">fidelity ✗</span>
                ))}
            </span>
          </Stat>
        )}
      </div>
    </section>
  );
}

function Stat({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="min-w-0">
      <div className="mb-0.5 text-[10.5px] tracking-[0.12em] text-faint uppercase">{label}</div>
      <div className="truncate text-[13px] text-ink">{children}</div>
    </div>
  );
}
