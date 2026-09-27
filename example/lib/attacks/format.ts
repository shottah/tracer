const usd = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  notation: "compact",
  maximumFractionDigits: 1,
});

/** "$197M", "$1.5B"; `null` (unknown) reads as "undisclosed". */
export function formatLossUsd(lossUsd: number | null): string {
  return lossUsd === null ? "undisclosed" : usd.format(lossUsd);
}
