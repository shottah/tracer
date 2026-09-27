import assert from "node:assert/strict";
import { test } from "node:test";
import { RECENT_MAX, parseRecent, pushRecent, serializeRecent, type RecentTx } from "./recent.ts";

const hash = (n: number) => `0x${n.toString(16).padStart(64, "0")}`;
const entry = (n: number, over: Partial<RecentTx> = {}): RecentTx => ({
  chain: "ethereum",
  hash: hash(n),
  ok: true,
  at: 1_700_000_000 + n,
  ...over,
});

test("round-trips through the cookie encoding", () => {
  const list = [
    entry(1, { method: "swapExactTokensForTokens", to: "Uniswap V2: Router" }),
    entry(2, { chain: "base-sepolia", ok: false }),
  ];
  assert.deepEqual(parseRecent(serializeRecent(list)), list);
});

test("a missing or malformed cookie reads as empty", () => {
  assert.deepEqual(parseRecent(undefined), []);
  assert.deepEqual(parseRecent(""), []);
  assert.deepEqual(parseRecent("not json"), []);
  assert.deepEqual(parseRecent(encodeURIComponent('{"c":"ethereum"}')), []);
});

test("drops entries that fail validation instead of the whole list", () => {
  const raw = encodeURIComponent(
    JSON.stringify([
      { c: "ethereum", h: hash(1), s: 1, t: 1 },
      { c: "ethereum", h: "0xnothex", s: 1, t: 2 },
      { c: "../etc", h: hash(3), s: 1, t: 3 },
      { c: "base", h: hash(4), s: 0, t: 4, m: 42 },
    ]),
  );
  assert.deepEqual(
    parseRecent(raw).map((e) => e.hash),
    [hash(1), hash(4)],
  );
  assert.equal(parseRecent(raw)[1].method, undefined);
});

test("push puts the visit first and de-duplicates by chain + hash", () => {
  const list = [entry(1), entry(2), entry(3)];
  const next = pushRecent(list, entry(2, { at: 9_999_999_999, method: "mint" }));
  assert.deepEqual(
    next.map((e) => e.hash),
    [hash(2), hash(1), hash(3)],
  );
  assert.equal(next[0].method, "mint");
  // Same hash on a different chain is a different entry.
  assert.equal(pushRecent(list, entry(1, { chain: "base" })).length, 4);
});

test("push caps the list and matches hashes case-insensitively", () => {
  let list: RecentTx[] = [];
  for (let i = 0; i < RECENT_MAX + 5; i++) list = pushRecent(list, entry(i));
  assert.equal(list.length, RECENT_MAX);
  assert.equal(list[0].hash, hash(RECENT_MAX + 4));

  const upper = pushRecent(
    [entry(0xabc)],
    entry(0xabc, { hash: hash(0xabc).toUpperCase().replace("0X", "0x") }),
  );
  assert.equal(upper.length, 1);
});

test("long labels are clipped so the cookie stays small", () => {
  const [e] = parseRecent(
    serializeRecent([entry(1, { to: "x".repeat(500), method: "y".repeat(500) })]),
  );
  assert.ok(e.to!.length <= 48);
  assert.ok(e.method!.length <= 48);
  const full = Array.from({ length: RECENT_MAX }, (_, i) =>
    entry(i, { to: "x".repeat(500), method: "y".repeat(500) }),
  );
  assert.ok(serializeRecent(full).length < 3500, "must fit well under the 4KB cookie limit");
});

test("non-ASCII labels can't push the encoded cookie past the budget", () => {
  const wide = "“🦄”".repeat(40);
  const full = Array.from({ length: RECENT_MAX }, (_, i) => entry(i, { to: wide, method: wide }));
  const encoded = serializeRecent(full);
  assert.ok(encoded.length <= 3500, `encoded ${encoded.length} bytes`);
  // Oldest entries go first; the newest visit always survives.
  const kept = parseRecent(encoded);
  assert.ok(kept.length >= 1);
  assert.equal(kept[0].hash, full[0].hash);
});

test("clipping never splits a surrogate pair", () => {
  const [e] = parseRecent(serializeRecent([entry(1, { to: "🦄".repeat(100) })]));
  assert.equal(e.to, "🦄".repeat(48));
});
