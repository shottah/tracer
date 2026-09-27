import assert from "node:assert/strict";
import { test } from "node:test";
import { CHAINS } from "../chains.ts";
import { HALL_OF_FAME, HallOfFameSource, attackPath, findAttack } from "./hall-of-fame.ts";
import { InvalidCursorError } from "./types.ts";

const source = new HallOfFameSource();

test("a page of 10 returns every attack and ends the feed", async () => {
  const page = await source.page(null, 10);
  assert.equal(page.items.length, HALL_OF_FAME.length);
  assert.equal(page.nextCursor, null);
});

test("small pages chain through offsets to the end", async () => {
  const first = await source.page(null, 2);
  assert.deepEqual(first.items.map((a) => a.id), ["euler-2023", "nomad-2022"]);
  assert.equal(first.nextCursor, "2");
  const second = await source.page(first.nextCursor, 2);
  assert.equal(second.nextCursor, "4");
  const third = await source.page(second.nextCursor, 2);
  assert.deepEqual(third.items.map((a) => a.id), ["balancer-v2-2025"]);
  assert.equal(third.nextCursor, null);
});

test("a cursor at the end yields an empty final page", async () => {
  const page = await source.page(String(HALL_OF_FAME.length), 10);
  assert.deepEqual(page, { items: [], nextCursor: null });
});

test("rejects cursors it could not have issued", async () => {
  for (const bad of ["-1", "abc", "99", "02", "1.0", " 1", "1e1", ""]) {
    await assert.rejects(source.page(bad, 10), InvalidCursorError, `cursor ${JSON.stringify(bad)}`);
  }
});

test("seed data is well-formed", () => {
  const ids = new Set(HALL_OF_FAME.map((a) => a.id));
  assert.equal(ids.size, HALL_OF_FAME.length, "ids are unique");
  const slugs = new Set(CHAINS.map((c) => c.slug));
  for (const a of HALL_OF_FAME) {
    assert.match(a.txHash, /^0x[0-9a-f]{64}$/, `${a.id} txHash`);
    assert.match(a.date, /^\d{4}-\d{2}-\d{2}$/, `${a.id} date`);
    assert.ok(slugs.has(a.chain), `${a.id} chain ${a.chain}`);
    assert.ok(a.summary.length > 0 && a.classification.length > 0, `${a.id} text`);
  }
  const losses = HALL_OF_FAME.map((a) => a.lossUsd ?? -1);
  assert.deepEqual(losses, [...losses].sort((x, y) => y - x), "sorted by loss, descending");
});

test("findAttack matches chain + hash, case-insensitively", () => {
  const euler = HALL_OF_FAME[0];
  assert.equal(findAttack(euler.chain, euler.txHash.toUpperCase().replace("0X", "0x")), euler);
  assert.equal(findAttack("base", euler.txHash), undefined);
  assert.equal(findAttack(euler.chain, `0x${"ab".repeat(32)}`), undefined);
});

test("attackPath is the inspector route for the attack", () => {
  const euler = HALL_OF_FAME[0];
  assert.equal(attackPath(euler), `/simulate/ethereum/${euler.txHash}`);
});
