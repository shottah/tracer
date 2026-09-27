"use client";

import { useEffect } from "react";
import {
  RECENT_COOKIE,
  RECENT_MAX_AGE,
  parseRecent,
  pushRecent,
  serializeRecent,
  type RecentTx,
} from "@/lib/recent";

export function readRecentCookie(): RecentTx[] {
  const raw = document.cookie
    .split("; ")
    .find((c) => c.startsWith(`${RECENT_COOKIE}=`))
    ?.slice(RECENT_COOKIE.length + 1);
  return parseRecent(raw);
}

export function writeRecentCookie(list: RecentTx[]) {
  const maxAge = list.length ? RECENT_MAX_AGE : 0;
  const secure = location.protocol === "https:" ? "; Secure" : "";
  document.cookie = `${RECENT_COOKIE}=${serializeRecent(list)}; Path=/; Max-Age=${maxAge}; SameSite=Lax${secure}`;
}

/**
 * Records a successfully traced transaction in the "recently inspected"
 * cookie. Client-side because Server Components can't set cookies.
 */
export function RememberTx({ visit }: { visit: Omit<RecentTx, "at"> }) {
  const { chain, hash, ok, method, to } = visit;
  useEffect(() => {
    const at = Math.floor(Date.now() / 1000);
    writeRecentCookie(pushRecent(readRecentCookie(), { chain, hash, ok, method, to, at }));
  }, [chain, hash, ok, method, to]);
  return null;
}
