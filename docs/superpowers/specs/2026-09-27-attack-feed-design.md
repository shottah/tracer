# Attack feed ("Hall of fame") — design

Status: approved in chat 2026-09-27, pending spec review. Scope: `example/` web
inspector only; no Rust or schema changes.

## Goal

The home page surfaces high-profile protocol attacks that a visitor can open
in the inspector with one click. The list loads as an infinite scroll in
batches of 10. For now the data is a hand-picked list of 5 attacks; the
design must let a live data source replace that list later without touching
the UI or the API route.

Non-goals (for this iteration): a live DeFiHackLabs/DefiLlama-backed source,
pre-warming trace reports, supporting chains beyond `lib/chains.ts`.

## Background: data sources

No free public feed carries both attack transaction hashes and reliable USD
losses:

- **DeFiHackLabs** (Apache-2.0): 861 PoCs, 576 with an explicit `Attack Tx`
  header; losses are free text. No machine-readable index.
- **DefiLlama `/hacks`**: clean USD loss + classification, no tx hashes.
  Name/date join to DeFiHackLabs matched only 63 of 325 ≥ $1M hacks on
  Ethereum/Arbitrum/Base.
- **BlockSec incident list**: has hashes, no public API. **De.Fi Rekt API**:
  paid, tx coverage unconfirmed.

Hence a hardcoded seed list behind a source interface; a future live source
would most likely wrap DeFiHackLabs.

## Seed data

All on Ethereum (`chain: "ethereum"`), each verified to trace end-to-end via
`tracer report --backend rpc` on dRPC. Loss and classification from
DefiLlama; dates from block timestamps (UTC). Sorted by loss, descending.

| id | protocol | date | lossUsd | classification | txHash |
|---|---|---|---|---|---|
| `euler-2023` | Euler Finance | 2023-03-13 | 197000000 | Donation attack | `0xc310a0affe2169d1f6feec1c63dbc7f7c62a887fa48795d327d4d2da2d6b111d` |
| `nomad-2022` | Nomad Bridge | 2022-08-01 | 190000000 | Forged proof | `0xa5fe9d044e4f3e5aa5bc4c0709333cd2190cba0f4e7f16bcf73f49f83e4a5460` |
| `beanstalk-2022` | Beanstalk | 2022-04-17 | 181000000 | Flash-loan governance | `0xcd314668aaa9bbfebaf1a0bd2b6553d01dd58899c508d4729fa7311dc5d33ad7` |
| `cream-2021` | Cream Finance | 2021-10-27 | 130000000 | Oracle manipulation | `0x0fe2542079644e107cbf13690eb9c2c65963ccb79089ff96bfaf8dced2331c92` |
| `balancer-v2-2025` | Balancer V2 | 2025-11-03 | 128000000 | Rounding error | `0x6ed07db1a9fe5c0794d44cd36081d6a6df103fab868cdd75d581e3bd23bc9742` |

One-line summaries (the `summary` field):

- Euler — `donateToReserves` let a leveraged account make itself insolvent,
  then liquidate itself at a discount.
- Nomad — an upgrade marked the zero root as trusted, so any unproven message
  passed; hundreds of copycats replayed this transaction.
- Beanstalk — flash-loaned governance power passed a malicious proposal via
  `emergencyCommit` in a single transaction.
- Cream — donating to yUSD inflated its price-per-share, letting the attacker
  borrow against overvalued collateral.
- Balancer V2 — rounding in composable stable pool swaps was compounded
  through `batchSwap` to deflate the pool invariant and drain it.

Notes: Beanstalk uses the exploit tx, not the proposal tx DeFiHackLabs links.
Radiant (Arbitrum, `0x1ce7e9a9…`) was dropped: dRPC's Arbitrum endpoint
reports it not found although it exists on-chain; tracked separately.

## Architecture

```
example/lib/attacks/
  types.ts         Attack, AttackPage, AttackSource
  hall-of-fame.ts  HallOfFameSource + the seed list
  feed.ts          PAGE_SIZE, attackSource(), getAttackPage() (cached)
example/app/api/attacks/route.ts   GET ?cursor= → AttackPage JSON
example/components/attack-feed.tsx client: cards + infinite scroll
example/app/page.tsx               renders page 1 below the feature tiles
```

### Contract (`types.ts`)

```ts
export interface Attack {
  id: string;             // stable, unique; React key
  protocol: string;
  date: string;           // ISO yyyy-mm-dd (UTC)
  chain: string;          // slug from lib/chains.ts
  txHash: string;         // 0x + 64 hex
  lossUsd: number | null; // null when unknown
  classification: string;
  summary: string;
}

export interface AttackPage {
  items: Attack[];
  nextCursor: string | null; // null = end of feed
}

export interface AttackSource {
  /** Throws InvalidCursorError for cursors it did not issue. */
  page(cursor: string | null, limit: number): Promise<AttackPage>;
}

export class InvalidCursorError extends Error {}
```

Cursors are opaque strings: an offset for the static source, a continuation
token for a future stream. Ordering is the source's concern.

### Static source (`hall-of-fame.ts`)

Cursor = decimal offset. `page(null, n)` returns items `[0, n)`;
`nextCursor` is the next offset, or `null` when the slice reaches the end.
A cursor that isn't a non-negative integer string within `[0, length]`
throws `InvalidCursorError`.

### Feed + caching (`feed.ts`)

- `PAGE_SIZE = 10`, fixed server-side; clients cannot pass a limit.
- `attackSource()` is the single swap point (returns `HallOfFameSource`).
- `getAttackPage(cursor)` = `unstable_cache(fn, ["attacks-page"],
  { revalidate: 3600, tags: ["attacks"] })`, keyed by cursor via the
  function argument. We use `unstable_cache`, not `"use cache"`: the latter
  requires `cacheComponents`, which conflicts with the home page's
  `force-dynamic`.
- `InvalidCursorError` must not be cached (it throws, so it isn't).

### Route (`app/api/attacks/route.ts`)

- `GET /api/attacks?cursor=<c>` → `200 AttackPage` with
  `Cache-Control: public, s-maxage=3600, stale-while-revalidate=86400`
  (served from App Hosting's CDN).
- `InvalidCursorError` → `400 { error }`, `Cache-Control: no-store`.
- Any other failure → `502 { error: "attack feed unavailable" }`,
  `no-store`, no upstream detail leaked.

### Home page

`app/page.tsx` calls `getAttackPage(null)` and renders
`<AttackFeed initial={page} />` in a "Hall of fame" section **below** the
existing three feature tiles. If the call throws, the section is omitted;
the rest of the page renders normally.

### `AttackFeed` (client)

- Renders cards: protocol, date, chain chip, loss (compact USD, e.g.
  "$197M"; "undisclosed" when null), classification tag, summary. The card
  is a `next/link` to `/simulate/<chain>/<txHash>`.
- If `nextCursor` is non-null, an `IntersectionObserver` sentinel after the
  last card fetches `/api/attacks?cursor=…` and appends. One request in
  flight at a time; stops when `nextCursor` is null.
- Loading state: a row below the cards. Error state: a row with a Retry
  button that re-requests the same cursor.
- Styling follows the existing tiles (`border-hairline`, `bg-panel/80`,
  mono accents).

Chain availability is not filtered: if the server can't serve the attack's
chain, the simulate page already explains that.

## Testing

- `node --test` unit tests in `example/lib/attacks/*.test.ts` (Node ≥ 22.18
  strips types natively; no new dependencies), added as `npm test`. Node
  needs explicit extensions, so modules under `lib/attacks/` that tests load
  (`types.ts`, `hall-of-fame.ts`, and `lib/chains.ts`) import each other
  with relative `.ts` paths, and `tsconfig.json` gains
  `"allowImportingTsExtensions": true` (valid because `noEmit` is set).
  `feed.ts` imports `next/cache` and is not loaded by tests. Tests:
  - pagination: `page(null, 10)` returns all 5 with `nextCursor: null`;
    `page(null, 2)` → `"2"`, then `"4"`, then `null`; invalid cursors
    (`"-1"`, `"abc"`, `"99"`) throw `InvalidCursorError`.
  - data sanity: ids unique, hashes match `TX_HASH` shape, chains exist in
    `CHAINS`, sorted by `lossUsd` descending, dates are ISO.
- `feed.ts`/route are thin wrappers over `unstable_cache`; verified by
  `tsc`, `eslint`, `next build`, and a manual run: home page shows the 5
  cards below the tiles, `/api/attacks?cursor=abc` returns 400,
  `/api/attacks` returns the cache header, and the Euler card opens a
  successful trace.

## Future: live source

A `DeFiHackLabsSource` implementing `AttackSource` (README for incident
list, PoC headers for tx hashes, optional DefiLlama join for USD) replaces
`HallOfFameSource` in `attackSource()`. Its fetches would use
`next: { revalidate }` and it would issue its own cursor format. Nothing
else changes.
