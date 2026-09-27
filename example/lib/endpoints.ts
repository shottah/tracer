/**
 * Which RPC endpoint serves which chain. With `DRPC_API_KEY` set every chain
 * in `CHAINS` is reachable through dRPC's one-key URL scheme; otherwise the
 * single `ETH_RPC_URL` serves whatever chain it reports via `eth_chainId`.
 */

import { CHAINS, chainById, type Chain } from "./chains";

export interface Endpoint {
  chain: Chain;
  url: string;
}

export type RpcMode =
  | { kind: "drpc"; key: string }
  | { kind: "url"; url: string }
  | { kind: "none" };

export function rpcMode(): RpcMode {
  const key = process.env.DRPC_API_KEY?.trim();
  if (key) return { kind: "drpc", key };
  const url = process.env.ETH_RPC_URL?.trim();
  if (url) return { kind: "url", url };
  return { kind: "none" };
}

export function drpcUrl(slug: string, key: string): string {
  return `https://lb.drpc.live/${slug}/${encodeURIComponent(key)}`;
}

/** Strip the configured secrets from text that may echo a request URL. */
export function redactSecrets(text: string): string {
  let out = text;
  const key = process.env.DRPC_API_KEY?.trim();
  if (key) out = out.split(key).join("***");
  const url = process.env.ETH_RPC_URL?.trim();
  if (url) out = out.split(url).join(safeHost(url) ?? "***");
  return out;
}

/** Scheme + host only; the path and query may embed an API key. */
export function safeHost(url: string): string | undefined {
  try {
    const u = new URL(url);
    return `${u.protocol}//${u.host}`;
  } catch {
    return undefined;
  }
}

// The chain behind ETH_RPC_URL never changes for a running server.
let urlChain: { url: string; chain: Promise<Chain> } | null = null;

function chainOfUrl(url: string): Promise<Chain> {
  if (urlChain?.url === url) return urlChain.chain;
  const chain = rpc<string>(url, "eth_chainId", []).then((hex) => chainById(Number(hex)));
  chain.catch(() => {
    if (urlChain?.url === url) urlChain = null; // retry on the next request
  });
  urlChain = { url, chain };
  return chain;
}

/** All reachable endpoints. Throws when `ETH_RPC_URL` can't be queried. */
export async function endpoints(): Promise<Endpoint[]> {
  const mode = rpcMode();
  switch (mode.kind) {
    case "drpc":
      return CHAINS.map((chain) => ({ chain, url: drpcUrl(chain.slug, mode.key) }));
    case "url":
      return [{ chain: await chainOfUrl(mode.url), url: mode.url }];
    case "none":
      return [];
  }
}

export interface Located {
  hits: Chain[];
  /** Chains whose lookup errored — the transaction might be on one of them. */
  failed: { chain: Chain; message: string }[];
}

/** Look `hash` up on every endpoint in parallel. */
export async function locateTx(hash: string, eps: Endpoint[]): Promise<Located> {
  const results = await Promise.allSettled(
    eps.map((ep) => rpc<unknown>(ep.url, "eth_getTransactionByHash", [hash])),
  );
  const located: Located = { hits: [], failed: [] };
  results.forEach((r, i) => {
    const chain = eps[i].chain;
    if (r.status === "rejected") {
      located.failed.push({ chain, message: redactSecrets(String(r.reason?.message ?? r.reason)) });
    } else if (r.value != null) {
      located.hits.push(chain);
    }
  });
  return located;
}

async function rpc<T>(url: string, method: string, params: unknown[]): Promise<T> {
  const res = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
    signal: AbortSignal.timeout(15_000),
    cache: "no-store",
  });
  const body = (await res.json().catch(() => ({}))) as {
    result?: T;
    error?: { message?: string };
  };
  if (!res.ok || body.error) {
    const detail = body.error?.message;
    if (res.ok) throw new Error(detail ?? "rpc error");
    throw new Error(`HTTP ${res.status}${detail ? `: ${detail}` : ""}`);
  }
  return body.result as T;
}
