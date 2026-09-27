# tracer example — web inspector

A small [Next.js](https://nextjs.org) app that drives the `tracer` CLI and
renders its JSON report as a Phalcon/Tenderly-style transaction inspector:
**invocation flow**, **balance changes**, and **fund flow**.

It's a thin, illustrative consumer of tracer's JSON contract
([docs/formats.md](../docs/formats.md)) — the report is generated server-side
by shelling out to the `tracer` binary; the UI is pure presentation. No
report logic is reimplemented here.

## How it works

```
browser ──▶ /simulate/<hash>          finds the chain (eth_getTransactionByHash on each)
                 │  redirects
                 ▼
            /simulate/<chain>/<hash> (server component)
                 │  spawns: tracer report <hash> --rpc-url <chain's endpoint> --compact [--deep]
                 ▼
            TraceReport JSON ──▶ React views (invocation tree · balances · React Flow graph)
```

- **`lib/chains.ts`** lists the supported chains (Ethereum, Base, Arbitrum
  One, and their Sepolia testnets); **`lib/endpoints.ts`** maps each to an
  RPC URL and locates a hash across them.
- **`lib/tracer.ts`** resolves the binary (`TRACER_BIN`, then
  `../target/{release,debug}/tracer`, then `PATH`), runs it, and caches each
  successful report (a mined transaction's report is immutable).
- **`lib/types.ts`** mirrors the JSON schema; **`lib/format.ts`** holds
  client-safe display helpers (address shortening, asset colors, amounts).
- **`app/simulate/[ref]/page.tsx`** (`/simulate/<hash>`) locates the chain
  and redirects; **`app/simulate/[ref]/[hash]/page.tsx`**
  (`/simulate/<chain>/<hash>`) renders the report. Invalid hashes 404;
  unknown transactions render a graceful "not found"; a missing RPC config
  renders setup guidance.
- The three views live in `components/` — `invocation-flow.tsx` (line-numbered
  call tree with events interleaved by execution `position`, kind chips,
  decoded calls, storage toggles, search), `balance-changes.tsx`, and
  `fund-flow.tsx` (React Flow + dagre LR layout consuming `fundFlow` JSON
  directly — **not** a Mermaid diagram).

## Setup

**1. Build the tracer binary** (from the repo root):

```sh
cargo build --release -p tracer-cli
```

**2. Configure the RPC endpoint:**

```sh
cd example
cp .env.example .env.local
# edit .env.local — set DRPC_API_KEY (all chains) or ETH_RPC_URL (one chain)
```

Any plain RPC works. When the endpoint lacks `debug_traceTransaction`,
tracer transparently falls back to a local **anvil fork** (requires
[Foundry](https://getfoundry.sh) on `PATH`). To trace a **local anvil node
directly** with `--deep`, start it with `anvil --steps-tracing`.

**3. Run:**

```sh
npm install
npm run dev
# open http://localhost:3000, paste a tx hash, or go straight to
# http://localhost:3000/simulate/0x<hash> (any chain)
# http://localhost:3000/simulate/base/0x<hash> (a specific chain)
```

## Environment

| Variable | Required | Purpose |
| --- | --- | --- |
| `DRPC_API_KEY` | one of these | [dRPC](https://drpc.org) key; enables every chain in `lib/chains.ts` via `https://lb.drpc.live/<chain>/<key>`. Wins over `ETH_RPC_URL` |
| `ETH_RPC_URL` | one of these | a single JSON-RPC endpoint; its chain comes from `eth_chainId` |
| `TRACER_BACKEND` | no | force `auto` / `rpc` / `anvil-fork` (use `rpc` on serverless) |
| `TRACER_BIN` | no | explicit path to the `tracer` binary |
| `TRACER_DEEP` | no | `1` (default) runs `--deep`; `0` disables. Per-request override: `?deep=1` / `?deep=0` |
| `LABELS_FILE` | no | path to the address-labels file (default `./labels.json`) |

## Labeling unverified contracts (`labels.json`)

Contracts that aren't ABI-verified on-chain render as bare addresses. Drop a
`labels.json` next to the app (copy [`labels.example.json`](labels.example.json))
mapping addresses to names:

```json
{
  "0x1a9ad59713b85750ef2f9cd8433f898a65c654a4": "VFE",
  "0x04a929e264165a0036ca8e317aeba471d5637d55": "USD Curve Pool"
}
```

Labels apply across every view — fund-flow nodes, the invocation tree,
balance-change rows, and the header — and **win over** anything tracer
derived (built-in labels, token symbols). For tokens without an on-chain
`symbol()`, the label also stands in for the ticker on fund-flow edges.
Address keys are case-insensitive; edits take effect on the next page load
(no restart needed). The file is gitignored — it's deployment-specific.

## Hall of fame

The home page lists high-profile protocol attacks below the feature tiles;
each card opens the attack transaction in the inspector. The list comes from
an `AttackSource` (`lib/attacks/types.ts`); today that's a hand-picked,
hardcoded set (`lib/attacks/hall-of-fame.ts`). To plug in a live source,
implement `AttackSource` (opaque cursors, `InvalidCursorError` for bad ones)
and return it from `attackSource()` in `lib/attacks/feed.ts`.

Pages of 10 are cached for an hour (`unstable_cache`, tag `attacks`) and
`/api/attacks?cursor=…` responses carry CDN cache headers. Nothing calls
`revalidateTag` today, and it wouldn't purge the CDN copy — after a data
change, expect up to an hour (plus the CDN's stale-while-revalidate window)
before every visitor sees it. Run the unit tests with `npm test` (Node ≥
22.18, which runs TypeScript natively).

## Deploying to Vercel

The app deploys as a normal Next.js project with one twist: the serverless
function needs the Rust `tracer` binary. The pieces that make that work:

- **Build-time binary fetch** — `npm run build` runs
  [`scripts/fetch-tracer.mjs`](scripts/fetch-tracer.mjs) before `next build`,
  downloading the fully static `x86_64-unknown-linux-musl` asset from this
  repo's GitHub release into `bin/tracer` (static musl runs on Vercel's
  Amazon Linux runtime, unlike glibc builds). Pin a release with
  `TRACER_VERSION`.
- **Function bundling** — `outputFileTracingIncludes` in
  [`next.config.ts`](next.config.ts) ships `bin/tracer` inside the function
  bundle; the bridge resolves it at `./bin/tracer` and restores the exec bit
  if a copy step dropped it.
- **Runtime limits** — the `/simulate/<chain>/<hash>` page exports
  `maxDuration = 300`; reports are cached in function memory (per warm
  instance).

Deploy from `example/`:

```sh
vercel link                                  # create/link the project
vercel env add DRPC_API_KEY production       # or ETH_RPC_URL for one chain
vercel env add TRACER_BACKEND production     # → rpc
vercel env add LABELS_JSON production        # optional: inline labels.json
vercel deploy --prod
```

Serverless constraints to know:

- **`TRACER_BACKEND=rpc` is required in spirit**: there is no anvil on
  Vercel, so the endpoint must support `debug_traceTransaction`
  (`https://sepolia.base.org` does, as do Alchemy/QuickNode debug tiers).
  Setting it makes unsupported endpoints fail with a clear message instead
  of attempting the anvil fallback.
- **Labels** — `labels.json` is gitignored, so deployed instances read
  `LABELS_JSON` (same shape, inline) instead; env entries win over the file.

## Deploying to Firebase App Hosting

Same build as Vercel (`npm run build` fetches the musl binary, bundled via
`outputFileTracingIncludes`), served on Cloud Run. Config lives in
[`apphosting.yaml`](apphosting.yaml) (instance sizing, `TRACER_BACKEND=rpc`,
`DRPC_API_KEY` from Secret Manager) and [`firebase.json`](firebase.json)
(backend `tracer`). [`.firebaserc`](.firebaserc) points at the maintainer's
project — switch it with `firebase use --add`.

Deploy from `example/` (the Firebase CLI looks for `firebase.json` in the
current directory):

```sh
firebase apphosting:secrets:set DRPC_API_KEY  # dRPC key (paid tier for debug_*)
firebase deploy --only apphosting
```

The same constraints as Vercel apply: the endpoint must support
`debug_traceTransaction` (Alchemy's free tier does not).

## Notes

- This is an **example**, not a hardened product: reports are cached in
  memory per server process, there's no auth/rate-limiting, and the route
  invokes a local binary — run it behind your own controls if exposed.
- Built with the App Router, React Flow (`@xyflow/react`), and dagre.
