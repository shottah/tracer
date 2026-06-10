/** Client-safe display helpers over the report JSON. */

import type {
  AmountJson,
  AssetJson,
  SignedAmountJson,
  TraceReport,
} from "./types";

export function shortAddr(addr: string): string {
  if (addr.length <= 12) return addr;
  return `${addr.slice(0, 6)}…${addr.slice(-4)}`;
}

export function shortHex(hex: string, max = 18): string {
  if (hex.length <= max) return hex;
  return `${hex.slice(0, max - 7)}…${hex.slice(-6)}`;
}

/** Case-insensitive label lookup (labels are keyed by checksummed address). */
export function labelFor(report: TraceReport, addr: string): string | undefined {
  const labels = report.addressLabels;
  if (!labels) return undefined;
  const direct = labels[addr];
  if (direct) return direct;
  const lower = addr.toLowerCase();
  for (const [k, v] of Object.entries(labels)) {
    if (k.toLowerCase() === lower) return v;
  }
  return undefined;
}

export function displayName(report: TraceReport, addr: string): string {
  return labelFor(report, addr) ?? shortAddr(addr);
}

export function assetSymbol(report: TraceReport, asset: AssetJson): string {
  if (asset.type === "native") return report.nativeSymbol;
  const token = report.tokens?.[asset.token];
  const base =
    token?.symbol ??
    Object.entries(report.tokens ?? {}).find(
      ([k]) => k.toLowerCase() === asset.token.toLowerCase(),
    )?.[1]?.symbol ??
    shortAddr(asset.token);
  if (asset.type === "erc721" || asset.type === "erc1155") {
    return `${base} #${asset.tokenId}`;
  }
  return base;
}

export function amountText(amount: AmountJson): string {
  return amount.formatted ?? amount.dec;
}

export function signedAmountText(amount: SignedAmountJson): string {
  const v = amount.formatted ?? amount.dec;
  return amount.negative || v.startsWith("-") ? v : `+${v}`;
}

/** Stable key for grouping/coloring by asset. */
export function assetColorKey(asset: AssetJson): string {
  return asset.type === "native" ? "native" : asset.token.toLowerCase();
}

/**
 * Semantic palette: native flows are green (Phalcon-style); each token gets
 * a stable hue from a small high-contrast palette.
 */
const TOKEN_PALETTE = [
  "#5b9cf6", // blue
  "#22d3ee", // cyan
  "#a78bfa", // violet
  "#f5b454", // amber
  "#f472b6", // pink
  "#93c47d", // sage
];

export function assetColor(asset: AssetJson): string {
  if (asset.type === "native") return "#34d399";
  const key = asset.token.toLowerCase();
  let h = 0;
  for (let i = 2; i < key.length; i++) h = (h * 31 + key.charCodeAt(i)) >>> 0;
  return TOKEN_PALETTE[h % TOKEN_PALETTE.length];
}

export function weiToEth(wei: string | number, decimals = 18): string {
  const v = BigInt(wei);
  if (v === 0n) return "0";
  const base = 10n ** BigInt(decimals);
  const int = v / base;
  const frac = (v % base).toString().padStart(decimals, "0").replace(/0+$/, "");
  return frac ? `${int}.${frac}` : int.toString();
}

export function gweiText(wei: number): string {
  return `${weiToEth(Math.trunc(wei), 9)} gwei`;
}

export function timestampText(ts?: number): string | undefined {
  if (!ts) return undefined;
  return new Date(ts * 1000).toISOString().replace("T", " ").replace(".000Z", " UTC");
}

export function txKindBadge(report: TraceReport, addr: string): "sender" | "receiver" | undefined {
  const lower = addr.toLowerCase();
  if (lower === report.tx.from.toLowerCase()) return "sender";
  const to = report.tx.to ?? report.tx.contractCreated;
  if (to && lower === to.toLowerCase()) return "receiver";
  return undefined;
}
