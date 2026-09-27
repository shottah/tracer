/**
 * Hand-picked high-profile attacks, each verified to trace end-to-end on
 * Ethereum via dRPC. Loss and classification from DefiLlama's hacks dataset;
 * dates from block timestamps (UTC). Sorted by loss, descending.
 */

import { InvalidCursorError, type Attack, type AttackPage, type AttackSource } from "./types.ts";

export const HALL_OF_FAME: readonly Attack[] = [
  {
    id: "euler-2023",
    protocol: "Euler Finance",
    date: "2023-03-13",
    chain: "ethereum",
    txHash: "0xc310a0affe2169d1f6feec1c63dbc7f7c62a887fa48795d327d4d2da2d6b111d",
    lossUsd: 197_000_000,
    classification: "Donation attack",
    summary:
      "donateToReserves let a leveraged account make itself insolvent, then liquidate itself at a discount.",
  },
  {
    id: "nomad-2022",
    protocol: "Nomad Bridge",
    date: "2022-08-01",
    chain: "ethereum",
    txHash: "0xa5fe9d044e4f3e5aa5bc4c0709333cd2190cba0f4e7f16bcf73f49f83e4a5460",
    lossUsd: 190_000_000,
    classification: "Forged proof",
    summary:
      "An upgrade marked the zero root as trusted, so any unproven message passed; hundreds of copycats replayed this transaction.",
  },
  {
    id: "beanstalk-2022",
    protocol: "Beanstalk",
    date: "2022-04-17",
    chain: "ethereum",
    txHash: "0xcd314668aaa9bbfebaf1a0bd2b6553d01dd58899c508d4729fa7311dc5d33ad7",
    lossUsd: 181_000_000,
    classification: "Flash-loan governance",
    summary:
      "Flash-loaned governance power passed a malicious proposal via emergencyCommit in a single transaction.",
  },
  {
    id: "cream-2021",
    protocol: "Cream Finance",
    date: "2021-10-27",
    chain: "ethereum",
    txHash: "0x0fe2542079644e107cbf13690eb9c2c65963ccb79089ff96bfaf8dced2331c92",
    lossUsd: 130_000_000,
    classification: "Oracle manipulation",
    summary:
      "Donating to yUSD inflated its price-per-share, letting the attacker borrow against overvalued collateral.",
  },
  {
    id: "balancer-v2-2025",
    protocol: "Balancer V2",
    date: "2025-11-03",
    chain: "ethereum",
    txHash: "0x6ed07db1a9fe5c0794d44cd36081d6a6df103fab868cdd75d581e3bd23bc9742",
    lossUsd: 128_000_000,
    classification: "Rounding error",
    summary:
      "Rounding in composable stable pool swaps, compounded through batchSwap, deflated the pool invariant and drained it.",
  },
];

/** Canonical non-negative integers only: "0", "12" — not "02", "1.0", " 1". */
const OFFSET_RE = /^(0|[1-9]\d*)$/;

/** Static source; the cursor is the decimal offset of the next item. */
export class HallOfFameSource implements AttackSource {
  async page(cursor: string | null, limit: number): Promise<AttackPage> {
    let start = 0;
    if (cursor !== null) {
      start = OFFSET_RE.test(cursor) ? Number(cursor) : -1;
      if (start < 0 || start > HALL_OF_FAME.length) throw new InvalidCursorError(cursor);
    }
    const end = Math.min(start + limit, HALL_OF_FAME.length);
    return {
      items: HALL_OF_FAME.slice(start, end),
      nextCursor: end < HALL_OF_FAME.length ? String(end) : null,
    };
  }
}
