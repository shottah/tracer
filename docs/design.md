# tracer — design

_2026-06-09. Status: implemented for v0.1._

An open-source, headless alternative to Phalcon Explorer and Tenderly's transaction
inspection features, written in Rust. Architecture borrows key learnings from
[OpenTracer](https://github.com/jeffchen006/OpenTracer) (Python): collect
`debug_traceTransaction` output, interpret the struct-log stream into semantic call
trees with storage tracking, and derive higher-level views (token transfers, balance
changes, fund flows) from the normalized trace.

## Goals (v0.1)

1. Modular crate layout — each layer usable as a library.
2. Faithful call-trace reconstruction (geth `callTracer` fast path **and** an
   OpenTracer-style struct-log interpreter for storage/ordering depth).
3. Trace via a local **anvil fork** so users don't need a `debug_*`-enabled remote
   node — a plain state RPC is enough; anvil re-executes the block locally.
4. **Balance changes** (Phalcon/Tenderly feature): per-account native + token deltas,
   gas-aware, headless JSON suitable for direct UI rendering.
5. **Fund flow** (Phalcon/Tenderly feature): ordered transfer graph (nodes/edges),
   headless JSON + Mermaid/DOT renderings.
6. Distributable: tag-driven GitHub release workflow producing static binaries;
   `cargo install --git` also works.

Out of scope for v0.1: web UI, USD pricing, arbitrary simulation (only existing tx
hashes), Parity/trace_* style traces, non-EVM chains.

## Crate layout

```
tracer-core      data model + ABI/event decoding + labels (pure, alloy-primitives only)
tracer-trace     callTracer normalizer + struct-log interpreter (pure)
tracer-analysis  transfers, balance changes, fund flow (pure)
tracer-client    backends (remote RPC / anvil fork), enrichment, orchestration (IO)
tracer-cli       binary: trace | balance-changes | fund-flow | report
```

Pure crates never touch the network — everything below `tracer-client` is unit-testable
with fixtures, mirroring OpenTracer's fetch/parse/analyze package split.

## Tracing pipeline

```
tx hash ──► backend ──► artifacts ──► normalize ──► analyze ──► TraceReport (JSON)
            (rpc | anvil-fork)        (Frame tree)   (transfers → balances, fund flow)
```

**Artifacts** per tx: `eth_getTransaction*`, receipt, block header, `callTracer`
(`withLog: true`), `prestateTracer` (`diffMode: true`, optional), struct logs
(`--deep`, optional, memory enabled).

**Backends**

- `rpc`: call `debug_traceTransaction` directly on the given endpoint.
- `anvil-fork`: spawn `anvil --fork-url <rpc> --fork-block-number B-1 --no-mining
  --order fifo --auto-impersonate`, pin the next block env (timestamp, base fee,
  coinbase, gas limit) to block B, replay block B's preceding transactions via
  impersonated `eth_sendTransaction`, submit the target tx, mine once, then run all
  tracers against anvil. Metadata in the report stays the *original* tx; a fidelity
  check (status / gasUsed / log count vs. the original receipt) is embedded so
  consumers can detect replay divergence. `--no-replay` skips preceding txs for speed.
- `auto` (default): try `rpc`; on "method not found/unsupported" fall back to
  `anvil-fork`.

Known fidelity limits (documented, surfaced as warnings): blob (type-3) transactions
replay without sidecars (`BLOBHASH` reads zero), `PREVRANDAO` is anvil's, and forking
old blocks needs an archive-state RPC (recent blocks work on any full node).

**Why both `callTracer` and struct logs?** `callTracer` gives exact gas/value/revert
data cheaply; struct logs (the OpenTracer approach) add per-frame storage reads/writes
and exact event/call interleaving. In `--deep` mode both run and are merged by tree
shape, with `callTracer` authoritative on amounts. Struct-log interpretation handles
depth-stable calls (precompiles/EOAs), DELEGATECALL context addresses for LOG/SSTORE
attribution, CREATE address capture from the parent's resume stack, and revert-reason
extraction from memory.

## Analysis

**Transfers** are the shared substrate. Sources: tx value, `CALL` value, `CREATE`
endowment, `SELFDESTRUCT` sweeps (native); ERC-20/721 `Transfer`,
ERC-1155 `TransferSingle/Batch` (expanded), and WETH `Deposit`/`Withdrawal` (only from
the chain's canonical wrapped-native address, to avoid false positives) decoded from
logs. Transfers inside reverted subtrees are excluded (an "effective" flag is carried
down the walk). Ordering: logs carry `position` (number of sibling subcalls preceding
them), so a single DFS interleaves calls and events into one global execution order —
without `position` data we fall back to receipt `logIndex` order with a warning.

**Balance changes**: native deltas come from `prestateTracer` diff when available
(exact, includes gas); otherwise derived from transfers plus
`gasUsed × effectiveGasPrice` for the sender and the priority fee for the coinbase
(`source: "prestate" | "derived"` is recorded). Token deltas aggregate transfers per
`(account, asset)`; ERC-721 tracks per-id ownership counts, ERC-1155 per-id amounts.

**Fund flow**: nodes = addresses touched by transfers (kind: eoa/contract/token,
labels applied), edges = individual transfers with execution order preserved —
deliberately not pre-aggregated so UIs can choose.

All amounts serialize three ways: `raw` (0x hex), `dec` (decimal string), `formatted`
(decimals-applied, filled when token metadata enrichment is on). JSON is camelCase,
versioned via `schemaVersion`. Full schema in [formats.md](formats.md).

## Verification strategy

Hermetic e2e against a **local offline anvil**: tests hand-assemble EVM bytecode
(an ERC-20-event emitter, a reverter, an orchestrator that sends ETH, triggers a
token transfer, survives an inner revert, and writes storage), submit a tx, then run
the full pipeline and assert the tree shape, transfer set, and that derived balance
deltas equal `eth_getBalance` ground truth. A second test forks the first anvil to
exercise the replay backend and cross-checks it against the direct-RPC result. Tests
skip gracefully when `anvil` isn't installed; CI installs Foundry so they always run
there. Pure layers (interpreter, analyzers, decoders) carry fixture-based unit tests.

## Alternatives considered

- **REVM in-process re-execution** instead of anvil: faster and no subprocess, but
  reimplements fork state fetching/caching that anvil already does well, and ties us
  to revm's release cadence. Anvil is the explicit milestone; revm inspector backend
  is a clean future addition behind the same artifact interface.
- **trace_* (Parity) APIs**: narrower node support than geth-style `debug_*`; skipped.
- **Receipt-logs-only transfers** (no trace): misses native flows entirely; rejected.
- **cargo-dist** for releases: excellent, but a hand-rolled ~100-line matrix workflow
  has zero toolchain lock-in and is easier to audit; revisit if distribution needs
  grow (installers, homebrew tap).
