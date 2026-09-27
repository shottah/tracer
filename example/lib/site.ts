/**
 * Public identity of the deployment, for metadata, robots.txt, the sitemap
 * and share cards. `SITE_URL` is read at build time too (robots and the
 * sitemap are prerendered), so hosted deployments must expose it to both.
 */

export const SITE_NAME = "tracer";

export const SITE_DESCRIPTION =
  "Free, open-source EVM transaction inspector: trace any Ethereum, Base or Arbitrum " +
  "transaction's call tree, token balance changes and fund flow. A Phalcon/Tenderly alternative.";

/** Origin without a trailing slash, e.g. `https://tracer.shottah.xyz`. */
export function siteUrl(): string {
  const raw = process.env.SITE_URL?.trim() || "http://localhost:3000";
  return raw.replace(/\/+$/, "");
}

/** JSON-LD for a `<script type="application/ld+json">`; `<` escaped per the Next guide. */
export function jsonLd(data: object): string {
  return JSON.stringify(data).replace(/</g, "\\u003c");
}
