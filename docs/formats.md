# JSON output formats

Everything `tracer` produces is designed to be rendered directly by a UI.
All JSON is **camelCase**. The envelope carries `schemaVersion` (currently
`"0.1"`); breaking shape changes bump it.

Amounts appear in triplicate so consumers never need big-number math:

```json
{ "raw": "0x14d1120d7b160000", "dec": "1500000000000000000", "formatted": "1.5" }
```

- `raw` — hex quantity (exact)
- `dec` — decimal string (exact; U256 does not fit in JSON numbers)
- `formatted` — decimals-applied display value, present when token metadata
  enrichment ran (native amounts always format with 18 decimals)

Signed amounts add `"negative": true|false`, and `dec`/`formatted` carry a
leading `-` when negative.

## Envelope (`tracer report`)

```json
{
  "schemaVersion": "0.1",
  "chainId": 1,
  "nativeSymbol": "ETH",
  "tx": {
    "hash": "0x…", "from": "0x…", "to": "0x…",
    "value": "0x0", "input": "0x…", "nonce": 5,
    "gasLimit": 210000, "gasUsed": 134001, "effectiveGasPrice": 12000000001,
    "status": true, "txType": 2,
    "blockNumber": 19000000, "blockTimestamp": 1705000000,
    "transactionIndex": 42, "baseFeePerGas": 12000000000
  },
  "backend": {
    "kind": "rpc" | "anvilFork",
    "endpointHost": "https://eth-mainnet.example.com",
    "fork": {
      "forkBlock": 18999999,
      "replayed": 41, "skipped": 0,
      "replayedTxHash": "0x…",
      "fidelity": { "statusMatch": true, "gasUsedMatch": true, "logCountMatch": true }
    }
  },
  "trace": { … },
  "transfers": [ … ],
  "balanceChanges": { … },
  "fundFlow": { … },
  "tokens": { "0xToken…": { "address": "0x…", "standard": "erc20", "symbol": "WETH", "name": "Wrapped Ether", "decimals": 18 } },
  "addressLabels": { "0xc02a…": "WETH" },
  "warnings": [ "…non-fatal degradations…" ]
}
```

Notes:

- `endpointHost` is scheme+host only — API keys embedded in RPC URLs never
  reach a report.
- `backend.fork` is present only for the anvil-fork backend. `fidelity`
  compares the replayed receipt with the original; if any flag is `false`,
  a warning explains the divergence.
- `warnings` is the contract for graceful degradation: consumers should
  surface them, not fail.

## Call trace (`trace`)

A tree of frames. `id` is a stable DFS pre-order index (referenced by
transfer origins); `parent` and `depth` make flattening trivial.

```json
{
  "id": 0, "depth": 0, "kind": "CALL",
  "from": "0x…", "to": "0x…",
  "value": "0x3e8", "gas": 500000, "gasUsed": 74000,
  "input": "0x…", "output": "0x…",
  "error": "execution reverted",        // absent when the frame succeeded
  "revertReason": "nope",               // decoded Error(string)/Panic(uint256)
  "decoded": {
    "selector": "0xa9059cbb",
    "name": "transfer",
    "signature": "transfer(address,uint256)",
    "params": [ { "name": "to", "value": "0x…" }, { "name": "amount", "value": "5" } ]
  },
  "logs": [
    {
      "address": "0x…", "topics": ["0xddf2…"], "data": "0x…",
      "position": 1,                    // subcalls preceding this log → exact interleaving
      "logIndex": 42,                   // matched to the receipt
      "decoded": { "name": "Transfer", "signature": "Transfer(address,address,uint256)", "params": [ … ] }
    }
  ],
  "storageReads":  [ { "slot": "0x…07", "value": "0x…00" } ],          // --deep only
  "storageWrites": [ { "slot": "0x…07", "previous": "0x…00", "value": "0x…2a" } ],
  "children": [ … ]
}
```

`kind` is one of `CALL`, `STATICCALL`, `DELEGATECALL`, `CALLCODE`, `CREATE`,
`CREATE2`, `SELFDESTRUCT` (matching geth). For `DELEGATECALL` frames, log
`address` is the caller's context, exactly as on chain.

## Transfers

The shared substrate for balance changes and fund flow — one entry per asset
movement, in execution order. Transfers inside reverted subtrees are
excluded.

```json
{
  "order": 2,
  "from": "0x…", "to": "0x…",
  "asset": { "type": "erc20", "token": "0x…" },
  "amount": { "raw": "0x5", "dec": "5", "formatted": "0.000005" },
  "origin": { "kind": "log", "logIndex": 42 }
}
```

`asset.type`: `native` | `erc20` | `erc721` (+`tokenId`) | `erc1155`
(+`tokenId`; batches are expanded into one transfer per id).
`origin.kind`: `txValue` | `call` | `create` | `selfDestruct` (with
`frameId` into the trace) | `log` | `deposit` | `withdrawal` (with
`logIndex`). `deposit`/`withdrawal` are wrapped-native wrap/unwrap flows,
honored only for the chain's canonical wrapper to avoid false positives.

## Balance changes (`balance-changes`)

```json
{
  "nativeSource": "prestate" | "derived",
  "gasIncluded": true,
  "changes": [
    {
      "address": "0x…",
      "label": "WETH",
      "native": {
        "pre": "0x…", "post": "0x…",              // prestate source only
        "delta": { "raw": "0x…", "negative": true, "dec": "-1500021…", "formatted": "-1.500021" },
        "gasFee": { "raw": "0x…", "dec": "21000…", "formatted": "0.000021" }   // sender row
      },
      "tokens": [
        { "asset": { "type": "erc20", "token": "0x…" }, "delta": { … }, "transferCount": 3 }
      ]
    }
  ]
}
```

- `nativeSource: "prestate"` means exact pre/post balances from
  `prestateTracer` (gas inherently included). `"derived"` means deltas were
  reconstructed from value transfers plus `gasUsed × effectiveGasPrice`
  (sender) and the priority fee (coinbase).
- The sender row is always first; remaining rows sort by address.
- ERC-721 deltas count ownership per token id (`±1`); ERC-1155 deltas are
  per-id amounts.

## Fund flow (`fund-flow`)

```json
{
  "nodes": [
    { "id": "0x…", "label": "Uniswap V2: Router02", "kind": "eoa" | "contract" | "token" | "account" }
  ],
  "edges": [
    {
      "id": 0, "order": 0,
      "from": "0x…", "to": "0x…",
      "asset": { "type": "native" },
      "amount": { … },
      "origin": { "kind": "txValue" }
    }
  ]
}
```

Edges are deliberately **one per transfer** (not aggregated) with execution
order preserved, so UIs can draw numbered arrows Phalcon-style or aggregate
however they like. `kind: "account"` means the address was only ever a
counterparty — the trace gives no evidence whether it has code.

## CLI ↔ JSON mapping

| Command                       | Output                                              |
| ----------------------------- | --------------------------------------------------- |
| `report`                      | full envelope (everything above)                    |
| `trace --format json`         | envelope with `trace` + `transfers` only            |
| `balance-changes --format json` | envelope with `balanceChanges` only               |
| `fund-flow --format json`     | envelope with `fundFlow` + `transfers` only         |

Every subcommand also has human renderings (`tree`, `table`, `mermaid`,
`dot`); the JSON is the stable interface.
