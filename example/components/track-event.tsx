"use client";

import { useEffect, useRef } from "react";
import { track, type AnalyticsEvent } from "@/lib/analytics";

/**
 * Fires one GA4 event when rendered — lets Server Components (the simulate
 * pages) report outcomes only they know, like trace success or failure.
 */
export function TrackEvent({ event }: { event: AnalyticsEvent }) {
  const key = JSON.stringify(event);
  // Dev StrictMode runs effects twice; don't double-count the same event.
  const sent = useRef<string | null>(null);
  useEffect(() => {
    if (sent.current === key) return;
    sent.current = key;
    track(JSON.parse(key) as AnalyticsEvent);
  }, [key]);
  return null;
}
