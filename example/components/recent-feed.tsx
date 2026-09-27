"use client";

import Link from "next/link";
import { useState } from "react";
import { Chip } from "@/components/ui";
import { writeRecentCookie } from "@/components/remember-tx";
import { chainBySlug } from "@/lib/chains";
import type { RecentTx } from "@/lib/recent";

/**
 * Home-page "Recently inspected": transactions this browser traced, read
 * server-side from the cookie so the list is in the first paint.
 */
export function RecentFeed({ initial }: { initial: RecentTx[] }) {
  const [items, setItems] = useState(initial);
  if (items.length === 0) return null;

  return (
    <section className="flex w-full flex-col gap-2.5">
      <div className="flex items-baseline justify-between">
        <h2 className="font-mono text-[11.5px] tracking-wider text-faint uppercase">
          Recently inspected
        </h2>
        <button
          type="button"
          onClick={() => {
            writeRecentCookie([]);
            setItems([]);
          }}
          className="cursor-pointer font-mono text-[11px] text-faint hover:text-ink"
        >
          clear
        </button>
      </div>
      <div className="flex flex-col divide-y divide-hairline rounded-md border border-hairline bg-panel/80">
        {items.map((e) => (
          <RecentRow key={`${e.chain}/${e.hash}`} tx={e} />
        ))}
      </div>
    </section>
  );
}

function RecentRow({ tx }: { tx: RecentTx }) {
  return (
    <Link
      href={`/simulate/${tx.chain}/${tx.hash}`}
      className="group flex flex-col gap-1 px-3.5 py-2.5 transition-colors first:rounded-t-md last:rounded-b-md hover:bg-panel-2"
    >
      <div className="flex items-baseline justify-between gap-3">
        <span className="min-w-0 truncate text-[13px] text-ink group-hover:text-accent">
          {tx.method ?? "call"}
          {tx.to && <span className="text-dim"> → {tx.to}</span>}
        </span>
        <span suppressHydrationWarning className="shrink-0 font-mono text-[11px] text-faint">
          {ago(tx.at)}
        </span>
      </div>
      <div className="flex min-w-0 items-center gap-1.5">
        <Chip tone={tx.ok ? "pos" : "neg"}>{tx.ok ? "success" : "failed"}</Chip>
        <Chip>{chainBySlug(tx.chain)?.name ?? tx.chain}</Chip>
        <span className="min-w-0 truncate font-mono text-[11px] text-faint">{tx.hash}</span>
      </div>
    </Link>
  );
}

function ago(unix: number): string {
  const s = Math.max(0, Math.floor(Date.now() / 1000) - unix);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
}
