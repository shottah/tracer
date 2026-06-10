/**
 * TypeScript mirror of tracer's JSON report contract (docs/formats.md).
 * All shapes are camelCase, versioned via `schemaVersion`.
 */

export interface AmountJson {
  raw: string;
  dec: string;
  formatted?: string;
}

export interface SignedAmountJson {
  raw: string;
  negative: boolean;
  dec: string;
  formatted?: string;
}

export type AssetJson =
  | { type: "native" }
  | { type: "erc20"; token: string }
  | { type: "erc721"; token: string; tokenId: string }
  | { type: "erc1155"; token: string; tokenId: string };

export type TransferOriginJson =
  | { kind: "txValue" }
  | { kind: "call"; frameId: number }
  | { kind: "create"; frameId: number }
  | { kind: "selfDestruct"; frameId: number }
  | { kind: "log"; logIndex?: number }
  | { kind: "deposit"; logIndex?: number }
  | { kind: "withdrawal"; logIndex?: number };

export type CallKindJson =
  | "CALL"
  | "STATICCALL"
  | "DELEGATECALL"
  | "CALLCODE"
  | "CREATE"
  | "CREATE2"
  | "SELFDESTRUCT";

export interface DecodedParamJson {
  name: string;
  value: string;
}

export interface DecodedCallJson {
  selector: string;
  name?: string;
  signature?: string;
  params?: DecodedParamJson[];
}

export interface DecodedEventJson {
  name: string;
  signature: string;
  params?: DecodedParamJson[];
}

export interface FrameLogJson {
  address: string;
  topics: string[];
  data: string;
  position?: number;
  logIndex?: number;
  decoded?: DecodedEventJson;
}

export interface StorageReadJson {
  slot: string;
  value?: string;
}

export interface StorageWriteJson {
  slot: string;
  previous?: string;
  value: string;
}

export interface FrameJson {
  id: number;
  parent?: number;
  depth: number;
  kind: CallKindJson;
  from: string;
  to?: string;
  value: string; // 0x-hex quantity
  gas: number;
  gasUsed: number;
  input: string;
  output: string;
  error?: string;
  revertReason?: string;
  decoded?: DecodedCallJson;
  logs?: FrameLogJson[];
  storageReads?: StorageReadJson[];
  storageWrites?: StorageWriteJson[];
  children?: FrameJson[];
}

export interface AssetTransferJson {
  order: number;
  from: string;
  to: string;
  asset: AssetJson;
  amount: AmountJson;
  origin: TransferOriginJson;
}

export interface NativeChangeJson {
  pre?: string;
  post?: string;
  delta: SignedAmountJson;
  gasFee?: AmountJson;
}

export interface TokenChangeJson {
  asset: AssetJson;
  delta: SignedAmountJson;
  transferCount: number;
}

export interface AccountBalanceChangeJson {
  address: string;
  label?: string;
  native?: NativeChangeJson;
  tokens?: TokenChangeJson[];
}

export interface BalanceChangesJson {
  nativeSource: "prestate" | "derived";
  gasIncluded: boolean;
  changes: AccountBalanceChangeJson[];
}

export type FlowNodeKind = "eoa" | "contract" | "token" | "account";

export interface FlowNodeJson {
  id: string;
  label?: string;
  kind: FlowNodeKind;
}

export interface FlowEdgeJson {
  id: number;
  order: number;
  from: string;
  to: string;
  asset: AssetJson;
  amount: AmountJson;
  origin: TransferOriginJson;
}

export interface FundFlowJson {
  nodes: FlowNodeJson[];
  edges: FlowEdgeJson[];
}

export interface TokenInfoJson {
  address: string;
  standard: "erc20" | "erc721" | "erc1155" | "unknown";
  symbol?: string;
  name?: string;
  decimals?: number;
}

export interface FidelityCheckJson {
  statusMatch: boolean;
  gasUsedMatch: boolean;
  logCountMatch: boolean;
}

export interface ForkInfoJson {
  forkBlock: number;
  replayed: number;
  skipped: number;
  replayedTxHash: string;
  fidelity?: FidelityCheckJson;
}

export interface BackendInfoJson {
  kind: "rpc" | "anvilFork";
  endpointHost?: string;
  fork?: ForkInfoJson;
}

export interface TxSummaryJson {
  hash: string;
  from: string;
  to?: string;
  contractCreated?: string;
  value: string;
  input: string;
  nonce: number;
  gasLimit: number;
  gasUsed: number;
  effectiveGasPrice: number;
  status: boolean;
  txType: number;
  blockNumber: number;
  blockTimestamp?: number;
  transactionIndex?: number;
  baseFeePerGas?: number;
}

export interface TraceReport {
  schemaVersion: string;
  chainId: number;
  nativeSymbol: string;
  tx: TxSummaryJson;
  backend: BackendInfoJson;
  trace?: FrameJson;
  transfers?: AssetTransferJson[];
  balanceChanges?: BalanceChangesJson;
  fundFlow?: FundFlowJson;
  tokens?: Record<string, TokenInfoJson>;
  addressLabels?: Record<string, string>;
  warnings?: string[];
}
