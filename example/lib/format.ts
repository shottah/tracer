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

/**
 * Case-insensitive label lookup. Reports patched by labels.json store
 * lowercase keys (fast path); raw reports key by checksummed address, so a
 * scan remains as the fallback.
 */
export function labelFor(report: TraceReport, addr: string): string | undefined {
  const labels = report.addressLabels;
  if (!labels) return undefined;
  const lower = addr.toLowerCase();
  const direct = labels[addr] ?? labels[lower];
  if (direct) return direct;
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
  // On-chain symbol first (it's the ticker); a labels.json name beats the
  // bare address for tokens that expose no metadata.
  const token = report.tokens?.[asset.token];
  const base =
    token?.symbol ??
    Object.entries(report.tokens ?? {}).find(
      ([k]) => k.toLowerCase() === asset.token.toLowerCase(),
    )?.[1]?.symbol ??
    labelFor(report, asset.token) ??
    shortAddr(asset.token);
  if (asset.type === "erc721" || asset.type === "erc1155") {
    return `${base} #${asset.tokenId}`;
  }
  return base;
}

export function amountText(amount: AmountJson): string {
  return amount.formatted ?? amount.dec;
}

/**
 * Compact a (possibly 18-decimal) amount string for an on-canvas graph label.
 * Full precision is preserved everywhere else (detail bar, balance table);
 * this is display-only, so float rounding is acceptable.
 *
 *   "123.937992234639736623" → "123.938"
 *   "0.000000000000001"       → "1.00e-15"
 *   "1234567.89"              → "1,234,567.89"
 */
export function compactAmount(value: string): string {
  const n = Number(value);
  if (!Number.isFinite(n)) {
    return value.length > 12 ? `${value.slice(0, 12)}…` : value;
  }
  if (n === 0) return "0";
  const abs = Math.abs(n);
  if (abs >= 1e12 || abs < 1e-6) return n.toExponential(2);
  if (abs >= 1000) return n.toLocaleString("en-US", { maximumFractionDigits: 2 });
  const s = n.toPrecision(abs >= 1 ? 6 : 4);
  return s.includes(".") && !s.includes("e") ? s.replace(/\.?0+$/, "") : s;
}

export function amountTextCompact(amount: AmountJson): string {
  return compactAmount(amountText(amount));
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
