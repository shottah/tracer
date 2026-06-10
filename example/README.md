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

## Notes

- This is an **example**, not a hardened product: reports are cached in
  memory per server process, there's no auth/rate-limiting, and the route
  invokes a local binary — run it behind your own controls if exposed.
- Built with the App Router, React Flow (`@xyflow/react`), and dagre.
