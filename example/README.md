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
browser ──▶ /simulate/[hash] (server component)
                 │  spawns: tracer report <hash> --rpc-url $ETH_RPC_URL --compact [--deep]
                 ▼
            TraceReport JSON ──▶ React views (invocation tree · balances · React Flow graph)
```

- **`lib/tracer.ts`** resolves the binary (`TRACER_BIN`, then
  `../target/{release,debug}/tracer`, then `PATH`), runs it, and caches each
  successful report (a mined transaction's report is immutable).
- **`lib/types.ts`** mirrors the JSON schema; **`lib/format.ts`** holds
  client-safe display helpers (address shortening, asset colors, amounts).
- **`app/simulate/[hash]/page.tsx`** is the route. Invalid hashes 404;
  unknown transactions render a graceful "not found"; a missing
  `ETH_RPC_URL` renders setup guidance.
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
# edit .env.local — set ETH_RPC_URL to any JSON-RPC endpoint
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
# http://localhost:3000/simulate/0x<hash>
```

## Environment

| Variable | Required | Purpose |
| --- | --- | --- |
| `ETH_RPC_URL` | yes | JSON-RPC endpoint tracer reads from |
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

## Deploying to Vercel

The app deploys as a normal Next.js project with one twist: the serverless
function needs the Rust `tracer` binary. The pieces that make that work:

- **Build-time binary fetch** — [`vercel.json`](vercel.json) runs
  [`scripts/fetch-tracer.mjs`](scripts/fetch-tracer.mjs) before `next build`,
  downloading the fully static `x86_64-unknown-linux-musl` asset from this
  repo's GitHub release into `bin/tracer` (static musl runs on Vercel's
  Amazon Linux runtime, unlike glibc builds). Pin a release with
  `TRACER_VERSION`.
- **Function bundling** — `outputFileTracingIncludes` in
  [`next.config.ts`](next.config.ts) ships `bin/tracer` inside the function
  bundle; the bridge resolves it at `./bin/tracer` and restores the exec bit
  if a copy step dropped it.
- **Runtime limits** — the `/simulate/[hash]` page exports
  `maxDuration = 300`; reports are cached in function memory (per warm
  instance).

Deploy from `example/`:

```sh
vercel link                                  # create/link the project
vercel env add ETH_RPC_URL production        # a debug-capable RPC endpoint
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

## Notes

- This is an **example**, not a hardened product: reports are cached in
  memory per server process, there's no auth/rate-limiting, and the route
  invokes a local binary — run it behind your own controls if exposed.
- Built with the App Router, React Flow (`@xyflow/react`), and dagre.
