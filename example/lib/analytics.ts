/**
 * GA4 custom events. A no-op unless NEXT_PUBLIC_GA_ID is set (the root
 * layout only loads the tag then). Client-only: call from event handlers or
 * effects, never during render.
 *
 * Every event name and parameter here is mirrored as a GA4 custom
 * definition (Admin → Custom definitions); rename one and GA silently
 * starts a new, empty dimension.
 */

export type AnalyticsEvent =
  | { name: "trace_submit"; params: { chain: string; source: "form" | "crumb" } }
  | {
      name: "trace_view";
      params: { chain: string; deep: boolean; tx_status: "success" | "reverted"; calls: number };
    }
  | { name: "trace_error"; params: { chain: string; error_kind: TraceErrorKind } }
  | { name: "tab_view"; params: { tab: string } }
  | { name: "feed_click"; params: { feed: "attack" | "recent"; chain: string; protocol?: string } }
  | { name: "chain_select"; params: { chain: string } };

export type TraceErrorKind =
  | "not_found"
  | "trace_failed"
  | "rpc_unreachable"
  | "chain_unavailable";

type Gtag = (command: "event", name: string, params: Record<string, unknown>) => void;

const ENABLED = Boolean(process.env.NEXT_PUBLIC_GA_ID);
// The GA init script runs after hydration, so an effect on a freshly loaded
// page can fire before `window.gtag` exists; poll briefly instead of dropping.
const RETRY_MS = 100;
const MAX_WAIT_MS = 5000;

export function track(event: AnalyticsEvent, waited = 0): void {
  if (!ENABLED || typeof window === "undefined") return;
  const gtag = (window as unknown as { gtag?: Gtag }).gtag;
  if (gtag) {
    gtag("event", event.name, event.params);
  } else if (waited < MAX_WAIT_MS) {
    setTimeout(() => track(event, waited + RETRY_MS), RETRY_MS);
  }
}
