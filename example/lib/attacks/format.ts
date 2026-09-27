const usd = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  notation: "compact",
  // Pin both bounds: the currency default for the minimum varies across ICU
  // versions (Node 22 renders "$197.0M"), which would also skew SSR vs browser.
  minimumFractionDigits: 0,
  maximumFractionDigits: 1,
});

/** "$197M", "$1.5B"; `null` (unknown) reads as "undisclosed". */
export function formatLossUsd(lossUsd: number | null): string {
  return lossUsd === null ? "undisclosed" : usd.format(lossUsd);
}
