/**
 * Contract for the home-page attack feed. A source pages through attacks with
 * opaque cursors — an offset for the static list, a continuation token for a
 * future live source — so sources can be swapped without touching the UI.
 */

export interface Attack {
  /** Stable and unique; used as the React key. */
  id: string;
  protocol: string;
  /** ISO `yyyy-mm-dd`, UTC. */
  date: string;
  /** Chain slug from `lib/chains.ts`. */
  chain: string;
  txHash: string;
  /** `null` when the loss is unknown. */
  lossUsd: number | null;
  classification: string;
  /** One line on the root cause. */
  summary: string;
}

export interface AttackPage {
  items: Attack[];
  /** `null` at the end of the feed. */
  nextCursor: string | null;
}

export interface AttackSource {
  /** Throws `InvalidCursorError` for cursors it did not issue. */
  page(cursor: string | null, limit: number): Promise<AttackPage>;
}

export class InvalidCursorError extends Error {
  constructor(cursor: string) {
    super(`invalid cursor: ${JSON.stringify(cursor)}`);
    this.name = "InvalidCursorError";
  }
}
