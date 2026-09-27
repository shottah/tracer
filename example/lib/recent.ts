/**
 * "Recently inspected" history, kept in a cookie so the home page can render
 * it server-side. Written client-side after a successful trace
 * (`components/remember-tx.tsx`), read in `app/page.tsx`. Holds only public
 * data (chain + tx hash + display hints), never RPC details.
 */

export const RECENT_COOKIE = "tracer_recent";
export const RECENT_MAX = 8;
/** 30 days. */
export const RECENT_MAX_AGE = 60 * 60 * 24 * 30;

// Cookies cap at ~4KB and browsers silently drop larger ones (losing the whole
// history), so labels are clipped and the encoded list is held under a budget.
const LABEL_MAX = 48;
const ENCODED_MAX = 3500;
const HASH_RE = /^0x[0-9a-fA-F]{64}$/;
const SLUG_RE = /^[a-z0-9-]{1,32}$/;

export interface RecentTx {
  chain: string;
  hash: string;
  /** Receipt status. */
  ok: boolean;
  /** Visit time, unix seconds. */
  at: number;
  /** Decoded top-level function name (or "transfer" / "create"). */
  method?: string;
  /** Label of the `to` address, when the report had one. */
  to?: string;
}

/** Compact wire shape: short keys to save cookie bytes. */
interface Wire {
  c: string;
  h: string;
  s: 0 | 1;
  t: number;
  m?: string;
  l?: string;
}

/** Clip by code point so an emoji isn't split into a lone surrogate. */
const clip = (s: string | undefined) =>
  typeof s === "string" && s.length > 0 ? Array.from(s).slice(0, LABEL_MAX).join("") : undefined;

/**
 * Non-ASCII labels encode to up to 9 bytes per character, so a full list can
 * blow the budget; drop the oldest entries until it fits.
 */
export function serializeRecent(list: RecentTx[]): string {
  const wire: Wire[] = list.slice(0, RECENT_MAX).map((e) => ({
    c: e.chain,
    h: e.hash,
    s: e.ok ? 1 : 0,
    t: e.at,
    ...(clip(e.method) && { m: clip(e.method) }),
    ...(clip(e.to) && { l: clip(e.to) }),
  }));
  let encoded = encodeURIComponent(JSON.stringify(wire));
  while (encoded.length > ENCODED_MAX && wire.length > 1) {
    wire.pop();
    encoded = encodeURIComponent(JSON.stringify(wire));
  }
  return encoded;
}

/** Tolerant parse: a bad cookie yields `[]`, a bad entry is dropped. */
export function parseRecent(raw: string | undefined): RecentTx[] {
  if (!raw) return [];
  let data: unknown;
  try {
    data = JSON.parse(decodeURIComponent(raw));
  } catch {
    return [];
  }
  if (!Array.isArray(data)) return [];
  const out: RecentTx[] = [];
  for (const w of data as Partial<Wire>[]) {
    if (!w || typeof w !== "object") continue;
    if (typeof w.c !== "string" || !SLUG_RE.test(w.c)) continue;
    if (typeof w.h !== "string" || !HASH_RE.test(w.h)) continue;
    if (typeof w.t !== "number" || !Number.isFinite(w.t)) continue;
    out.push({
      chain: w.c,
      hash: w.h,
      ok: w.s === 1,
      at: w.t,
      ...(typeof w.m === "string" && { method: clip(w.m) }),
      ...(typeof w.l === "string" && { to: clip(w.l) }),
    });
    if (out.length === RECENT_MAX) break;
  }
  return out;
}

/** Most recent first; a revisit moves the entry to the front. */
export function pushRecent(list: RecentTx[], visit: RecentTx): RecentTx[] {
  const same = (e: RecentTx) =>
    e.chain === visit.chain && e.hash.toLowerCase() === visit.hash.toLowerCase();
  return [visit, ...list.filter((e) => !same(e))].slice(0, RECENT_MAX);
}
