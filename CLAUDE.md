# tracer — agent notes

Headless EVM transaction inspector (Phalcon/Tenderly alternative) in Rust.
Architecture and trade-offs: `docs/design.md`. JSON contract: `docs/formats.md`.

## Commands

```sh
cargo test --workspace                                 # unit + e2e (e2e auto-skips without anvil)
cargo clippy --workspace --all-targets -- -D warnings  # must stay clean
cargo fmt --all
cargo build --release -p tracer-cli                    # → target/release/tracer
```

E2E tests need Foundry's `anvil` on PATH (installed here and in CI). They
spawn their own nodes on random ports — safe to run in parallel.

## Crate map (dependency order)

- `tracer-core` — data model, sol!-based decoding, labels. Pure; only
  alloy-primitives/sol-types. The JSON shapes here are a versioned public
  contract (`SCHEMA_VERSION`, camelCase serde) — breaking changes bump it.
- `tracer-trace` — geth `callTracer` normalizer + struct-log interpreter
  (`structlog.rs` is the subtle one: stack operands are listed bottom-first,
  code-less callees don't bump depth, CREATE addresses come from the
  parent's resume stack).
- `tracer-analysis` — transfers / balance changes / fund flow. Pure
  functions; revert-awareness lives in `transfers::walk`.
- `tracer-client` — IO: rpc + anvil-fork backends, enrichment, orchestrator
  (`tracer.rs::report` is the end-to-end flow).
- `tracer-cli` — bin `tracer`; renderers in `src/render/`.
- `example/` — Next.js (App Router) web inspector; **not** a workspace
  member. Shells out to the `tracer` binary server-side (`lib/tracer.ts`),
  renders the three Phalcon-style views from the JSON. `lib/types.ts` mirrors
  `docs/formats.md` — keep them in sync when the schema changes. Fund-flow
  graph is React Flow + dagre (NOT mermaid). Its own CLAUDE.md/AGENTS.md warns
  this is Next 16 with breaking changes — read `node_modules/next/dist/docs/`.

## Gotchas

- anvil only emits struct logs when started with `--steps-tracing`; the
  fork backend passes it automatically, but direct-RPC `--deep` against a
  user's anvil needs it.
- `StructLog.op` is `Box<str>` in alloy 2 — match on `&*op`.
- alloy 2.x: providers/rpc-types are version 2, alloy-primitives/sol-types
  are version 1 — both pinned in `[workspace.dependencies]`.
- RPC URLs may embed API keys: anything user-facing must go through
  `rpc::redact_endpoint`.
- Releases are tag-driven (`RELEASING.md`); `release.yml` builds with
  `--locked`, so commit `Cargo.lock` changes.
