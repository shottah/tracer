"use client";

import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { Chip } from "@/components/ui";
import { track } from "@/lib/analytics";
import { formatLossUsd } from "@/lib/attacks/format";
import type { Attack, AttackPage } from "@/lib/attacks/types";
import { chainBySlug } from "@/lib/chains";

/**
 * Home-page "Hall of fame": attack cards linking to their traces. Page 1 is
 * server-rendered; later pages load from /api/attacks when the sentinel below
 * the last card scrolls into view.
 */
export function AttackFeed({ initial }: { initial: AttackPage }) {
  const [items, setItems] = useState<Attack[]>(initial.items);
  const [cursor, setCursor] = useState<string | null>(initial.nextCursor);
  const [status, setStatus] = useState<"idle" | "loading" | "error">("idle");
  const inFlight = useRef(false);
  const sentinel = useRef<HTMLDivElement>(null);

  const loadMore = useCallback(async () => {
    if (cursor === null || inFlight.current) return;
    inFlight.current = true;
    setStatus("loading");
    try {
      const res = await fetch(`/api/attacks?cursor=${encodeURIComponent(cursor)}`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const page = (await res.json()) as AttackPage;
      setItems((prev) => [...prev, ...page.items]);
      setCursor(page.nextCursor);
      setStatus("idle");
    } catch {
      setStatus("error");
    } finally {
      inFlight.current = false;
    }
  }, [cursor]);

  useEffect(() => {
    const el = sentinel.current;
    if (!el || cursor === null || status === "error") return;
    const io = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) void loadMore();
      },
      { rootMargin: "200px" },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [cursor, status, loadMore]);

  return (
    <div className="flex w-full flex-col gap-2.5">
      {items.map((a) => (
        <AttackCard key={a.id} attack={a} />
      ))}
      {cursor !== null && <div ref={sentinel} aria-hidden className="h-px" />}
      {status === "loading" && (
        <div className="py-2 text-center font-mono text-[11.5px] text-faint">loading…</div>
      )}
      {status === "error" && (
        <div className="flex items-center justify-center gap-3 py-2 font-mono text-[11.5px] text-neg">
          couldn’t load more attacks
          <button
            type="button"
            onClick={() => void loadMore()}
            className="cursor-pointer rounded border border-hairline-2 px-2 py-0.5 text-dim hover:text-ink"
          >
            Retry
          </button>
        </div>
      )}
    </div>
  );
}

function AttackCard({ attack: a }: { attack: Attack }) {
  return (
    <Link
      href={`/simulate/${a.chain}/${a.txHash}`}
      onClick={() =>
        track({
          name: "feed_click",
          params: { feed: "attack", chain: a.chain, protocol: a.protocol },
        })
      }
      className="group rounded-md border border-hairline bg-panel/80 px-3.5 py-3 transition-colors hover:border-accent/50"
    >
      <div className="flex items-baseline justify-between gap-3">
        <span className="text-[13px] font-medium text-ink group-hover:text-accent">
          {a.protocol}
        </span>
        <span className="shrink-0 font-mono text-[11px] text-faint">{a.date}</span>
      </div>
      <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
        <Chip tone="neg">{formatLossUsd(a.lossUsd)}</Chip>
        <Chip>{chainBySlug(a.chain)?.name ?? a.chain}</Chip>
        <Chip tone="violet">{a.classification}</Chip>
      </div>
      <p className="mt-1.5 text-[11.5px] leading-snug text-dim">{a.summary}</p>
    </Link>
  );
}
