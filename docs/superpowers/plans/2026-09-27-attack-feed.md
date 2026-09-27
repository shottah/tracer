# Attack Feed (Hall of Fame) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A "Hall of fame" section below the home-page feature tiles lists high-profile protocol attacks (infinite scroll, batches of 10), each linking to its trace in the inspector.

**Architecture:** An `AttackSource` interface with opaque cursors; a hardcoded `HallOfFameSource` (5 attacks) is the only implementation and is chosen in one place (`feed.ts`). `getAttackPage` wraps the source in `unstable_cache`; `/api/attacks` serves later pages with CDN cache headers; the home page server-renders page 1 and a client component fetches the rest on scroll.

**Tech Stack:** Next.js 16.2 App Router (`example/`, not a Cargo workspace member), React 19, Tailwind, Node ≥ 22.18 built-in test runner (`node --test`, native TS stripping).

**Spec:** `docs/superpowers/specs/2026-09-27-attack-feed-design.md`

## Global Constraints

- All work is inside `example/`; no Rust, schema, or `docs/formats.md` changes.
- No new npm dependencies.
- `PAGE_SIZE = 10`, fixed server-side; clients cannot pass a limit.
- Cache: `unstable_cache(..., ["attacks-page"], { revalidate: 3600, tags: ["attacks"] })`; route 200s send `Cache-Control: public, s-maxage=3600, stale-while-revalidate=86400`; 400/502 send `Cache-Control: no-store`.
- Do not use `"use cache"` (requires `cacheComponents`, conflicts with the home page's `force-dynamic`).
- Files under `example/lib/attacks/` import each other with relative `.ts` paths (Node needs extensions); `tsconfig.json` gets `"allowImportingTsExtensions": true`.
- The feed section renders **below** the three existing feature tiles.
- This is Next 16 — per `example/AGENTS.md`, check `example/node_modules/next/dist/docs/` before using an API you're unsure of.
- Follow existing style: semicolons, double quotes, 2-space indent, ~100-col lines (the repo does **not** run Prettier with default width — do not run `prettier --write`).

## Review Focus

- Non-canonical cursors (`"02"`, `"1.0"`, `" 1"`, `"1e1"`, `""` via the source) → rejected with `InvalidCursorError`, not silently coerced. Pinned in Task 1.
- Cursor exactly at the end (`"5"` for 5 items) → valid, empty page, `nextCursor: null`. Pinned in Task 1.
- `?cursor=` present but empty on the route → treated as the first page, not 400. Pinned in Task 2 (curl check).
- `InvalidCursorError` thrown through `unstable_cache` may not survive `instanceof` → route matches on `err.name` too. Pinned in Task 2 (curl `?cursor=abc` → 400).
- Sentinel intersecting twice before the first fetch resolves → exactly one request per cursor (in-flight ref guard). Pinned in Task 3 (component code + manual check with a temporary `PAGE_SIZE = 2`).

---

### Task 1: Attack contract, hall-of-fame source, loss formatting (+ unit tests)

**Files:**
- Create: `example/lib/attacks/types.ts`
- Create: `example/lib/attacks/hall-of-fame.ts`
- Create: `example/lib/attacks/format.ts`
- Test: `example/lib/attacks/hall-of-fame.test.ts`
- Test: `example/lib/attacks/format.test.ts`
- Modify: `example/tsconfig.json` (add `allowImportingTsExtensions`)
- Modify: `example/package.json` (add `test` script)

**Interfaces:**
- Consumes: `CHAINS` from `example/lib/chains.ts` (tests only).
- Produces:
  - `interface Attack { id: string; protocol: string; date: string; chain: string; txHash: string; lossUsd: number | null; classification: string; summary: string }`
  - `interface AttackPage { items: Attack[]; nextCursor: string | null }`
  - `interface AttackSource { page(cursor: string | null, limit: number): Promise<AttackPage> }`
  - `class InvalidCursorError extends Error` with `name === "InvalidCursorError"`
  - `const HALL_OF_FAME: readonly Attack[]`, `class HallOfFameSource implements AttackSource`
  - `function formatLossUsd(lossUsd: number | null): string`

- [ ] **Step 1: Enable `.ts` import extensions and the test script**

In `example/tsconfig.json`, add after `"noEmit": true,`:

```json
    "allowImportingTsExtensions": true,
```

In `example/package.json` `"scripts"`, add after `"lint": "eslint"` (add a comma to the previous line):

```json
    "test": "node --test \"lib/attacks/*.test.ts\""
```

- [ ] **Step 2: Write the contract (`types.ts`)**

```ts
/**
 * Contract for the home-page attack feed. A source pages through attacks with
 * opaque cursors — an offset for the static list, a continuation token for a
 * future live source — so sources can be swapped without touching the UI.
 */

export interface Attack {
  /** Stable and unique; used as the React key. */
  id: string;
  protocol: string;
  /** ISO `yyyy-mm-dd`, UTC. */
  date: string;
  /** Chain slug from `lib/chains.ts`. */
  chain: string;
  txHash: string;
  /** `null` when the loss is unknown. */
  lossUsd: number | null;
  classification: string;
  /** One line on the root cause. */
  summary: string;
}

export interface AttackPage {
  items: Attack[];
  /** `null` at the end of the feed. */
  nextCursor: string | null;
}

export interface AttackSource {
  /** Throws `InvalidCursorError` for cursors it did not issue. */
  page(cursor: string | null, limit: number): Promise<AttackPage>;
}

export class InvalidCursorError extends Error {
  constructor(cursor: string) {
    super(`invalid cursor: ${JSON.stringify(cursor)}`);
    this.name = "InvalidCursorError";
  }
}
```

- [ ] **Step 3: Write the failing tests**

`example/lib/attacks/hall-of-fame.test.ts`:

```ts
import assert from "node:assert/strict";
import { test } from "node:test";
import { CHAINS } from "../chains.ts";
import { HALL_OF_FAME, HallOfFameSource } from "./hall-of-fame.ts";
import { InvalidCursorError } from "./types.ts";

const source = new HallOfFameSource();

test("a page of 10 returns every attack and ends the feed", async () => {
  const page = await source.page(null, 10);
  assert.equal(page.items.length, HALL_OF_FAME.length);
  assert.equal(page.nextCursor, null);
});

test("small pages chain through offsets to the end", async () => {
  const first = await source.page(null, 2);
  assert.deepEqual(first.items.map((a) => a.id), ["euler-2023", "nomad-2022"]);
  assert.equal(first.nextCursor, "2");
  const second = await source.page(first.nextCursor, 2);
  assert.equal(second.nextCursor, "4");
  const third = await source.page(second.nextCursor, 2);
  assert.deepEqual(third.items.map((a) => a.id), ["balancer-v2-2025"]);
  assert.equal(third.nextCursor, null);
});

test("a cursor at the end yields an empty final page", async () => {
  const page = await source.page(String(HALL_OF_FAME.length), 10);
  assert.deepEqual(page, { items: [], nextCursor: null });
});

test("rejects cursors it could not have issued", async () => {
  for (const bad of ["-1", "abc", "99", "02", "1.0", " 1", "1e1", ""]) {
    await assert.rejects(source.page(bad, 10), InvalidCursorError, `cursor ${JSON.stringify(bad)}`);
  }
});

test("seed data is well-formed", () => {
  const ids = new Set(HALL_OF_FAME.map((a) => a.id));
  assert.equal(ids.size, HALL_OF_FAME.length, "ids are unique");
  const slugs = new Set(CHAINS.map((c) => c.slug));
  for (const a of HALL_OF_FAME) {
    assert.match(a.txHash, /^0x[0-9a-f]{64}$/, `${a.id} txHash`);
    assert.match(a.date, /^\d{4}-\d{2}-\d{2}$/, `${a.id} date`);
    assert.ok(slugs.has(a.chain), `${a.id} chain ${a.chain}`);
    assert.ok(a.summary.length > 0 && a.classification.length > 0, `${a.id} text`);
  }
  const losses = HALL_OF_FAME.map((a) => a.lossUsd ?? -1);
  assert.deepEqual(losses, [...losses].sort((x, y) => y - x), "sorted by loss, descending");
});
```

`example/lib/attacks/format.test.ts`:

```ts
import assert from "node:assert/strict";
import { test } from "node:test";
import { formatLossUsd } from "./format.ts";

test("formats losses as compact USD", () => {
  assert.equal(formatLossUsd(197_000_000), "$197M");
  assert.equal(formatLossUsd(1_500_000_000), "$1.5B");
  assert.equal(formatLossUsd(950_000), "$950K");
  assert.equal(formatLossUsd(0), "$0");
});

test("unknown losses read as undisclosed", () => {
  assert.equal(formatLossUsd(null), "undisclosed");
});
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `cd example && npm test`
Expected: FAIL — `Cannot find module '.../lib/attacks/hall-of-fame.ts'` (and `format.ts`).

- [ ] **Step 5: Implement the source (`hall-of-fame.ts`)**

```ts
/**
 * Hand-picked high-profile attacks, each verified to trace end-to-end on
 * Ethereum via dRPC. Loss and classification from DefiLlama's hacks dataset;
 * dates from block timestamps (UTC). Sorted by loss, descending.
 */

import { InvalidCursorError, type Attack, type AttackPage, type AttackSource } from "./types.ts";

export const HALL_OF_FAME: readonly Attack[] = [
  {
    id: "euler-2023",
    protocol: "Euler Finance",
    date: "2023-03-13",
    chain: "ethereum",
    txHash: "0xc310a0affe2169d1f6feec1c63dbc7f7c62a887fa48795d327d4d2da2d6b111d",
    lossUsd: 197_000_000,
    classification: "Donation attack",
    summary:
      "donateToReserves let a leveraged account make itself insolvent, then liquidate itself at a discount.",
  },
  {
    id: "nomad-2022",
    protocol: "Nomad Bridge",
    date: "2022-08-01",
    chain: "ethereum",
    txHash: "0xa5fe9d044e4f3e5aa5bc4c0709333cd2190cba0f4e7f16bcf73f49f83e4a5460",
    lossUsd: 190_000_000,
    classification: "Forged proof",
    summary:
      "An upgrade marked the zero root as trusted, so any unproven message passed; hundreds of copycats replayed this transaction.",
  },
  {
    id: "beanstalk-2022",
    protocol: "Beanstalk",
    date: "2022-04-17",
    chain: "ethereum",
    txHash: "0xcd314668aaa9bbfebaf1a0bd2b6553d01dd58899c508d4729fa7311dc5d33ad7",
    lossUsd: 181_000_000,
    classification: "Flash-loan governance",
    summary:
      "Flash-loaned governance power passed a malicious proposal via emergencyCommit in a single transaction.",
  },
  {
    id: "cream-2021",
    protocol: "Cream Finance",
    date: "2021-10-27",
    chain: "ethereum",
    txHash: "0x0fe2542079644e107cbf13690eb9c2c65963ccb79089ff96bfaf8dced2331c92",
    lossUsd: 130_000_000,
    classification: "Oracle manipulation",
    summary:
      "Donating to yUSD inflated its price-per-share, letting the attacker borrow against overvalued collateral.",
  },
  {
    id: "balancer-v2-2025",
    protocol: "Balancer V2",
    date: "2025-11-03",
    chain: "ethereum",
    txHash: "0x6ed07db1a9fe5c0794d44cd36081d6a6df103fab868cdd75d581e3bd23bc9742",
    lossUsd: 128_000_000,
    classification: "Rounding error",
    summary:
      "Rounding in composable stable pool swaps, compounded through batchSwap, deflated the pool invariant and drained it.",
  },
];

/** Canonical non-negative integers only: "0", "12" — not "02", "1.0", " 1". */
const OFFSET_RE = /^(0|[1-9]\d*)$/;

/** Static source; the cursor is the decimal offset of the next item. */
export class HallOfFameSource implements AttackSource {
  async page(cursor: string | null, limit: number): Promise<AttackPage> {
    let start = 0;
    if (cursor !== null) {
      start = OFFSET_RE.test(cursor) ? Number(cursor) : -1;
      if (start < 0 || start > HALL_OF_FAME.length) throw new InvalidCursorError(cursor);
    }
    const end = Math.min(start + limit, HALL_OF_FAME.length);
    return {
      items: HALL_OF_FAME.slice(start, end),
      nextCursor: end < HALL_OF_FAME.length ? String(end) : null,
    };
  }
}
```

- [ ] **Step 6: Implement the formatter (`format.ts`)**

```ts
const usd = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  notation: "compact",
  maximumFractionDigits: 1,
});

/** "$197M", "$1.5B"; `null` (unknown) reads as "undisclosed". */
export function formatLossUsd(lossUsd: number | null): string {
  return lossUsd === null ? "undisclosed" : usd.format(lossUsd);
}
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cd example && npm test`
Expected: PASS — 7 tests, 0 failures. (An `ExperimentalWarning` or module-type warning on stderr is fine; failures are not. If Node refuses the `.ts` files, check `node --version` ≥ 22.18.)

- [ ] **Step 8: Typecheck and lint**

Run: `cd example && npx tsc --noEmit && npx eslint lib/attacks`
Expected: no output, exit 0.

- [ ] **Step 9: Commit**

```bash
git add example/lib/attacks example/tsconfig.json example/package.json
git commit -m "feat(example): attack source contract and hall-of-fame seed list"
```

---

### Task 2: Cached feed and `/api/attacks` route

**Files:**
- Create: `example/lib/attacks/feed.ts`
- Create: `example/app/api/attacks/route.ts`

**Interfaces:**
- Consumes: `AttackPage`, `AttackSource`, `HallOfFameSource` from Task 1.
- Produces:
  - `const PAGE_SIZE = 10`
  - `function attackSource(): AttackSource`
  - `const getAttackPage: (cursor: string | null) => Promise<AttackPage>` (cached)
  - `function isInvalidCursor(err: unknown): boolean`
  - `GET /api/attacks?cursor=<c>` → `200 AttackPage` | `400 { error }` | `502 { error }`

- [ ] **Step 1: Implement `feed.ts`**

```ts
/**
 * The home-page attack feed. `attackSource()` is the single swap point: a live
 * source (e.g. DeFiHackLabs-backed) replaces the hardcoded list here and
 * nothing else changes.
 */

import { unstable_cache } from "next/cache";
import { HallOfFameSource } from "./hall-of-fame.ts";
import type { AttackPage, AttackSource } from "./types.ts";

export const PAGE_SIZE = 10;

export function attackSource(): AttackSource {
  return new HallOfFameSource();
}

/** One page of the feed, cached per cursor for an hour (tag: "attacks"). */
export const getAttackPage: (cursor: string | null) => Promise<AttackPage> = unstable_cache(
  (cursor: string | null) => attackSource().page(cursor, PAGE_SIZE),
  ["attacks-page"],
  { revalidate: 3600, tags: ["attacks"] },
);

/** `instanceof` may not survive the cache layer, so match on the name too. */
export function isInvalidCursor(err: unknown): boolean {
  return err instanceof Error && err.name === "InvalidCursorError";
}
```

- [ ] **Step 2: Implement the route (`app/api/attacks/route.ts`)**

```ts
import { getAttackPage, isInvalidCursor } from "@/lib/attacks/feed";

/** `GET /api/attacks?cursor=<c>` — the next page of the home-page attack feed. */
export async function GET(request: Request) {
  const cursor = new URL(request.url).searchParams.get("cursor") || null;
  try {
    const page = await getAttackPage(cursor);
    return Response.json(page, {
      headers: { "Cache-Control": "public, s-maxage=3600, stale-while-revalidate=86400" },
    });
  } catch (err) {
    const noStore = { "Cache-Control": "no-store" };
    if (isInvalidCursor(err)) {
      return Response.json({ error: (err as Error).message }, { status: 400, headers: noStore });
    }
    console.error("attack feed:", err);
    return Response.json({ error: "attack feed unavailable" }, { status: 502, headers: noStore });
  }
}
```

(`@/lib/attacks/feed` without an extension is fine here: this file is only ever bundled by Next.)

- [ ] **Step 3: Typecheck, lint, build**

Run: `cd example && npx tsc --noEmit && npx eslint lib/attacks app/api && npm run build`
Expected: build succeeds; the route table lists `ƒ /api/attacks`.

- [ ] **Step 4: Verify the route against a production server**

Run:

```bash
cd example && (PORT=3917 npx next start -p 3917 >/tmp/attacks.log 2>&1 &)
for i in $(seq 20); do curl -s -o /dev/null localhost:3917/api/attacks && break; sleep 1; done
curl -s -i localhost:3917/api/attacks | grep -i -E "^HTTP|cache-control"
curl -s localhost:3917/api/attacks | python3 -c "import json,sys;d=json.load(sys.stdin);print(len(d['items']),d['nextCursor'])"
curl -s -o /dev/null -w "%{http_code}\n" "localhost:3917/api/attacks?cursor="
curl -s -o /dev/null -w "%{http_code}\n" "localhost:3917/api/attacks?cursor=5"
curl -s -i "localhost:3917/api/attacks?cursor=abc" | grep -i -E "^HTTP|cache-control"
pkill -f "next start -p 3917"
```

Expected:
- `HTTP/1.1 200` with `cache-control: public, s-maxage=3600, stale-while-revalidate=86400`
- `5 None`
- `200` (empty cursor = first page)
- `200` (end cursor = empty page)
- `HTTP/1.1 400` with `cache-control: no-store`

If Next rewrites the 200's `Cache-Control`, stop and check `node_modules/next/dist/docs/01-app/01-getting-started/15-route-handlers.md` and `02-guides/cdn-caching.md` before changing anything.

- [ ] **Step 5: Commit**

```bash
git add example/lib/attacks/feed.ts example/app/api/attacks/route.ts
git commit -m "feat(example): cached attack feed and /api/attacks route"
```

---

### Task 3: `AttackFeed` component and home-page section

**Files:**
- Create: `example/components/attack-feed.tsx`
- Modify: `example/app/page.tsx` (make `Home` async, render the section after the feature-tile grid)

**Interfaces:**
- Consumes: `Attack`, `AttackPage` (Task 1), `formatLossUsd` (Task 1), `getAttackPage` (Task 2), `GET /api/attacks` (Task 2), `chainBySlug` from `@/lib/chains`, `Chip` from `@/components/ui`.
- Produces: `function AttackFeed({ initial }: { initial: AttackPage }): JSX.Element` (client component).

- [ ] **Step 1: Implement the component (`components/attack-feed.tsx`)**

```tsx
"use client";

import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { Chip } from "@/components/ui";
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
    const io = new IntersectionObserver((entries) => {
      if (entries.some((e) => e.isIntersecting)) void loadMore();
    }, { rootMargin: "200px" });
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
```

Before relying on `Chip`, open `example/components/ui.tsx` and confirm the `tone` values used (`"neg"`, `"violet"`, default `"dim"`) exist — they do as of this plan.

- [ ] **Step 2: Wire the section into `app/page.tsx`**

Add imports (keep the existing ones, alphabetical by path as the file does):

```tsx
import { AttackFeed } from "@/components/attack-feed";
import { getAttackPage } from "@/lib/attacks/feed";
import type { AttackPage } from "@/lib/attacks/types";
```

Make `Home` async and load page 1 at the top of its body, after `const binFound = …`:

```tsx
export default async function Home() {
```

```tsx
  // The feed is decoration: if its source fails, omit the section.
  let attacks: AttackPage | null = null;
  try {
    attacks = await getAttackPage(null);
  } catch (err) {
    console.error("attack feed:", err);
  }
```

Immediately after the closing `</div>` of the feature-tile grid (`<div className="grid w-full grid-cols-1 gap-2.5 sm:grid-cols-3">…</div>`), still inside the `rise` column, add:

```tsx
        {attacks && attacks.items.length > 0 && (
          <section className="flex w-full flex-col gap-2.5">
            <h2 className="font-mono text-[11.5px] uppercase tracking-wider text-faint">
              Hall of fame — replay a famous attack
            </h2>
            <AttackFeed initial={attacks} />
          </section>
        )}
```

Also change the `<main>` class so a long feed isn't vertically centered into the top edge: replace `justify-center` with `justify-center py-16`.

- [ ] **Step 3: Typecheck, lint, test, build**

Run: `cd example && npx tsc --noEmit && npx eslint && npm test && npm run build`
Expected: all pass; build succeeds.

- [ ] **Step 4: Manual check in the running app**

Start the app with a real key (from `example/.env.local`; it holds `ETH_RPC_URL=https://lb.drpc.live/<slug>/<key>` — use the dRPC key as `DRPC_API_KEY` so the Ethereum endpoint exists):

```bash
cd example
K=$(sed -n 's#^ETH_RPC_URL=.*/##p' .env.local)
(DRPC_API_KEY="$K" ETH_RPC_URL= PORT=3917 npx next start -p 3917 >/tmp/attacks.log 2>&1 &)
```

Open `http://localhost:3917/` (T3 preview tools or a browser) and verify:
- The 5 cards render **below** the three feature tiles, in order Euler, Nomad, Beanstalk, Cream, Balancer V2, each showing loss chip (e.g. `$197M`), `Ethereum` chip, classification chip, summary.
- No "loading…" row appears (5 < 10, `nextCursor` is null, so no request to `/api/attacks` is made — confirm in the network log / server log).
- Clicking **Euler Finance** opens `/simulate/ethereum/0xc310…` and the trace renders (≈2–5 s).

Then exercise the scroll path once: temporarily set `PAGE_SIZE = 2` in `lib/attacks/feed.ts`, rebuild, restart, scroll to the bottom and confirm the cards arrive as 2 + 2 + 1 with exactly one `/api/attacks` request per cursor (`?cursor=2`, `?cursor=4`) and no duplicates. **Revert `PAGE_SIZE` to 10** and confirm `git diff lib/attacks/feed.ts` is empty.

Stop the server: `pkill -f "next start -p 3917"`.

- [ ] **Step 5: Commit**

```bash
git add example/components/attack-feed.tsx example/app/page.tsx
git commit -m "feat(example): hall-of-fame attack feed on the home page"
```

---

### Task 4: Document the feed

**Files:**
- Modify: `example/README.md` (add a short section after the existing feature/usage description, before "Deploying")

- [ ] **Step 1: Add the section**

```markdown
## Hall of fame

The home page lists high-profile protocol attacks below the feature tiles;
each card opens the attack transaction in the inspector. The list comes from
an `AttackSource` (`lib/attacks/types.ts`); today that's a hand-picked,
hardcoded set (`lib/attacks/hall-of-fame.ts`). To plug in a live source,
implement `AttackSource` (opaque cursors, `InvalidCursorError` for bad ones)
and return it from `attackSource()` in `lib/attacks/feed.ts`.

Pages of 10 are cached for an hour (`unstable_cache`, tag `attacks`) and
`/api/attacks?cursor=…` responses carry CDN cache headers. Run the unit
tests with `npm test`.
```

- [ ] **Step 2: Commit**

```bash
git add example/README.md
git commit -m "docs(example): document the hall-of-fame attack feed"
```
