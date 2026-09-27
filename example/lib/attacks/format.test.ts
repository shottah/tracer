import assert from "node:assert/strict";
import { test } from "node:test";
import { formatLossUsd } from "./format.ts";

test("formats losses as compact USD", () => {
  assert.equal(formatLossUsd(197_000_000), "$197M");
  assert.equal(formatLossUsd(1_500_000_000), "$1.5B");
  assert.equal(formatLossUsd(950_000), "$950K");
  assert.equal(formatLossUsd(0), "$0");
});

test("unknown losses read as undisclosed", () => {
  assert.equal(formatLossUsd(null), "undisclosed");
});
