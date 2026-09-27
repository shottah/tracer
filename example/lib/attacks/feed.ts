/**
 * The home-page attack feed. `attackSource()` is the single swap point: a live
 * source (e.g. DeFiHackLabs-backed) replaces the hardcoded list here and
 * nothing else changes.
 */

import { unstable_cache } from "next/cache";
import { HallOfFameSource } from "./hall-of-fame.ts";
import type { AttackPage, AttackSource } from "./types.ts";

export const PAGE_SIZE = 10;

export function attackSource(): AttackSource {
  return new HallOfFameSource();
}

/** One page of the feed, cached per cursor for an hour (tag: "attacks"). */
export const getAttackPage: (cursor: string | null) => Promise<AttackPage> = unstable_cache(
  (cursor: string | null) => attackSource().page(cursor, PAGE_SIZE),
  ["attacks-page", "hall-of-fame", String(PAGE_SIZE)], // change "hall-of-fame" when attackSource() returns a different source
  { revalidate: 3600, tags: ["attacks"] },
);

/** Matches by name as well as instanceof, so sources that throw from another realm or bundle still map to a 400. */
export function isInvalidCursor(err: unknown): boolean {
  return err instanceof Error && err.name === "InvalidCursorError";
}
