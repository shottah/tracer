# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-06-09

Initial release.

### Added

- Call-trace reconstruction from geth `callTracer` with decoded functions,
  events, and revert reasons.
- OpenTracer-style struct-log interpreter (`--deep`): per-frame storage
  reads/writes, exact call/event interleaving, DELEGATECALL context
  attribution, CREATE/CREATE2 address recovery.
- Balance changes: per-account native + ERC-20/721/1155 deltas, WETH
  wrap/unwrap aware, gas-inclusive; exact via `prestateTracer` diff with a
  derived fallback.
- Fund-flow graph: ordered per-transfer edges with node classification,
  rendered as JSON, Mermaid, or DOT.
- Anvil-fork backend: trace through a local fork of any plain RPC — block
  env pinning, FIFO replay of preceding transactions via impersonation, and
  a fidelity check against the original receipt.
- `tracer` CLI: `trace`, `balance-changes`, `fund-flow`, `report` with
  human and JSON outputs; versioned camelCase JSON schema for UIs
  (`docs/formats.md`).
- Crates: `tracer-core`, `tracer-trace`, `tracer-analysis`,
  `tracer-client`, `tracer-cli`.
- Hermetic e2e suite against local anvil, including a fork-replay
  cross-check; CI and tag-driven multi-platform release workflow.
