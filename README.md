# tracer

**Headless EVM transaction inspector in Rust** — an open-source alternative to
[Phalcon Explorer](https://explorer.phalcon.xyz) and
[Tenderly](https://tenderly.co)'s transaction views.

Point it at any transaction hash and get the three views security engineers
and protocol teams actually use, as clean JSON (for your UI) or pretty
terminal output (for you):

- **Call trace** — the full internal call tree with decoded functions,
  events, revert reasons, and (in `--deep` mode) per-frame storage
  reads/writes reconstructed OpenTracer-style from raw struct logs.
- **Balance changes** — per-account native + token deltas (ERC-20/721/1155
  and WETH wrap/unwrap aware), gas-inclusive, exact when the node serves
  `prestateTracer` and derived otherwise.
- **Fund flow** — the ordered transfer graph behind Phalcon's diagram, as
  JSON nodes/edges, Mermaid, or DOT.

The killer feature: **you don't need a `debug`-enabled archive node**. With
`--backend anvil-fork` (or automatically in `auto` mode), tracer spawns a
local [anvil](https://getfoundry.sh) fork of any plain RPC, replays the
block up to your transaction with impersonated senders, re-executes it
locally, and traces *that* — with a fidelity check against the original
receipt embedded in the report.

Real output (`tracer trace <tx> --deep`, against a local anvil):

```text
tx      0xa4a518cb6b74324a6fdfacb2de46d5af8763a9518284dd787b51f79684719820
status  success   block 4   gas used 59498   type 2
fee     0.00003993731168173 ETH   (gas price 0.671237885 gwei)

CALL 0x9fe4…a6e0  {0.000000000000001 ETH}  gas=59498
├─ CALL 0x7099…79c8  {0.000000000000000001 ETH}
├─ CALL 0x5fbd…0aa3.transfer(to: 0x3C44…93BC, amount: 5)  gas=1799
│  └─ emit Transfer(from: 0x9fE4…a6e0, to: 0x3C44…93BC, value: 5)  @0x5fbd…0aa3
├─ CALL 0xe7f1…0512  gas=4  ✗ execution reverted
└─ sstore 0x0000000000000000000000…00000007 ← 0x0000000000000000000000…0000002a

transfers (3):
    1. 0xf39f…2266 → 0x9fe4…a6e0  0.000000000000001 ETH
    2. 0x9fe4…a6e0 → 0x7099…79c8  0.000000000000000001 ETH
    3. 0x9fe4…a6e0 → 0x3c44…93bc  5 0x5fbd…0aa3
```

## Install

**Prebuilt binaries** — grab the archive for your platform from the
[latest release](../../releases/latest), unpack, and put `tracer` on your
`PATH`.

**From source** (Rust 1.88+):

```sh
cargo install --git <this-repo-url> tracer-cli
# or, from a checkout:
cargo build --release -p tracer-cli   # → target/release/tracer
```

For the anvil-fork backend you also need [Foundry](https://getfoundry.sh)
(`anvil` on `PATH`).

## Quickstart

```sh
export ETH_RPC_URL=https://your-rpc-endpoint

# Human views
tracer trace 0x<txhash>
tracer balance-changes 0x<txhash>
tracer fund-flow 0x<txhash>                 # Mermaid by default
tracer fund-flow 0x<txhash> --format dot    # graphviz

# Headless: one JSON document with everything (schema in docs/formats.md)
tracer report 0x<txhash> -o report.json
```

Useful flags (all subcommands):

| Flag | Effect |
| --- | --- |
| `--backend auto\|rpc\|anvil-fork` | `auto` (default) tries the endpoint's debug API, falls back to a local anvil fork |
| `--deep` | adds struct-log interpretation: per-frame storage reads/writes + exact call/event interleaving |
| `--no-replay` | fork mode: skip replaying the block's preceding transactions (faster, less faithful) |
| `--no-enrich` | skip token `symbol`/`decimals` lookups and amount formatting |
| `--no-gas` | exclude gas fees from derived balance changes |
| `--format json` | machine output for any subcommand |
| `-o FILE` | write to a file instead of stdout |

Exit codes: `0` ok · `1` error · `2` transaction not found/not mined.
Warnings (degradations, fidelity notes) go to stderr; suppress with `-q`.

## How tracing works

```text
tx hash ──► backend ──► artifacts ──► normalize ──► analyze ──► TraceReport
            rpc | anvil-fork          Frame tree     transfers → balances, fund flow
```

- **rpc backend**: `debug_traceTransaction` with `callTracer` (+`withLog`),
  `prestateTracer` (diff mode, optional), and the default struct-log tracer
  (`--deep`, optional) straight against your endpoint.
- **anvil-fork backend**: forks the chain at `block - 1` from any plain RPC,
  pins the next block's env (timestamp, base fee, coinbase, gas limit),
  replays preceding transactions FIFO via `--auto-impersonate`, mines one
  block, then runs all tracers against anvil. The report keeps the
  *original* transaction's metadata and records a fidelity check
  (status/gasUsed/log-count vs. the real receipt).
- `--deep` runs an [OpenTracer](https://github.com/jeffchen006/OpenTracer)-style
  interpreter over raw struct logs — call tree, storage accesses,
  DELEGATECALL context attribution, CREATE address recovery, revert-reason
  decoding — and merges it into the `callTracer` tree (which stays
  authoritative for amounts and gas).

Known fidelity limits (always surfaced as report warnings): blob (type-3)
transactions replay without sidecars (`BLOBHASH` reads zero), `PREVRANDAO`
is anvil's, and forking old blocks needs an RPC that still serves state at
that height (recent blocks work on any full node; historical ones need an
archive endpoint — but *not* a debug/trace one).

Tracing a **local anvil node directly** (`--backend rpc`)? Start it with
`anvil --steps-tracing` if you want `--deep` data — without that flag anvil
returns empty struct logs.

## Use as a library

The CLI is a thin shell over five crates; everything below it is reusable:

```rust
use tracer_client::{Tracer, TracerConfig};

let tracer = Tracer::connect(TracerConfig {
    rpc_url: std::env::var("ETH_RPC_URL")?,
    ..Default::default()
})?;
let report = tracer.report(tx_hash).await?;   // tracer_core::TraceReport
serde_json::to_string_pretty(&report)?;       // ships straight to a UI
```

| Crate | What it is |
| --- | --- |
| `tracer-core` | data model, ABI/event decoding, labels — pure, no IO |
| `tracer-trace` | `callTracer` normalizer + struct-log interpreter — pure |
| `tracer-analysis` | transfers, balance changes, fund flow — pure |
| `tracer-client` | RPC + anvil-fork backends, enrichment, orchestration |
| `tracer-cli` | the `tracer` binary |

The pure layers are fixture-tested without a network; `tracer-client` is
verified end-to-end against a live local anvil — including a test that forks
one anvil from another and proves the replay path produces byte-identical
transfers and balance deltas to direct RPC tracing.

JSON shapes are documented in [docs/formats.md](docs/formats.md); design
notes and trade-offs in [docs/design.md](docs/design.md).

## Comparison

| Feature | tracer | Phalcon | Tenderly | OpenTracer |
| --- | --- | --- | --- | --- |
| Call trace w/ decoding | ✅ | ✅ | ✅ | ✅ |
| Balance changes | ✅ | ✅ | ✅ | — |
| Fund flow graph | ✅ | ✅ | ✅ | — |
| Storage read/write tracking | ✅ (`--deep`) | ✅ | ✅ | ✅ |
| Works without debug/archive RPC | ✅ (anvil fork) | n/a (hosted) | n/a (hosted) | ❌ |
| Headless / scriptable | ✅ JSON | ❌ | API (paid) | ✅ |
| Self-hosted / open source | ✅ MIT/Apache | ❌ | ❌ | ✅ |
| Language | Rust | — | — | Python |

## Development

```sh
cargo test --workspace          # unit + e2e (e2e auto-skips without anvil)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

Releases are tag-driven: see [RELEASING.md](RELEASING.md).

## Acknowledgements

The struct-log interpretation approach follows
[OpenTracer](https://github.com/jeffchen006/OpenTracer) (Zhang et al.) —
the insight that full transaction semantics can be reconstructed from
standard `debug_traceTransaction` output, no proprietary tracers required.
Built on [alloy](https://github.com/alloy-rs/alloy) and
[Foundry](https://github.com/foundry-rs/foundry).

## License

MIT or Apache-2.0, at your option.
