/**
 * Server-side bridge to the `tracer` CLI. Resolves the binary, runs
 * `tracer report <hash> --format-equivalent JSON`, and caches successful
 * reports per (hash, options) — a mined transaction's report is immutable.
 */

import { execFile } from "node:child_process";
import { accessSync, chmodSync, constants, existsSync } from "node:fs";
import path from "node:path";
import { promisify } from "node:util";
import type { TraceReport } from "./types";

const execFileAsync = promisify(execFile);

export const TX_HASH_RE = /^0x[0-9a-fA-F]{64}$/;

export type ReportResult =
  | { ok: true; report: TraceReport }
  | { ok: false; kind: "notFound" | "error"; message: string };

export interface ReportOptions {
  deep: boolean;
}

/**
 * Locate the tracer binary: env override, then workspace builds, then the
 * deploy-time download (`bin/tracer`, see scripts/fetch-tracer.mjs), then
 * PATH. Restores the executable bit if a copy step dropped it.
 */
export function resolveTracerBin(): string {
  const fromEnv = process.env.TRACER_BIN;
  if (fromEnv && existsSync(fromEnv)) return ensureExecutable(fromEnv);
  const roots = [path.resolve(process.cwd(), ".."), process.cwd()];
  for (const root of roots) {
    for (const profile of ["release", "debug"]) {
      const candidate = path.join(root, "target", profile, "tracer");
      if (existsSync(candidate)) return candidate;
    }
  }
  const fetched = path.join(process.cwd(), "bin", "tracer");
  if (existsSync(fetched)) return ensureExecutable(fetched);
  return "tracer"; // hope it's on PATH
}

function ensureExecutable(bin: string): string {
  try {
    accessSync(bin, constants.X_OK);
  } catch {
    try {
      chmodSync(bin, 0o755);
    } catch {
      // exec will surface a clearer error
    }
  }
  return bin;
}

export function rpcUrl(): string | undefined {
  return process.env.ETH_RPC_URL;
}

/** Default for `--deep` (storage + exact interleaving); query param overrides. */
export function deepDefault(): boolean {
  return process.env.TRACER_DEEP !== "0";
}

/**
 * Backend selection (`--backend`). On serverless deployments set
 * `TRACER_BACKEND=rpc`: there is no anvil to fall back to, and a debug-capable
 * RPC is required anyway — this turns the fallback path into a clear error.
 */
function backendArg(): string | undefined {
  const v = process.env.TRACER_BACKEND;
  return v && ["auto", "rpc", "anvil-fork"].includes(v) ? v : undefined;
}

const cache = new Map<string, Promise<ReportResult>>();

export function runReport(hash: string, opts: ReportOptions): Promise<ReportResult> {
  const key = `${hash.toLowerCase()}:${opts.deep ? "deep" : "fast"}`;
  const hit = cache.get(key);
  if (hit) return hit;
  const pending = runReportUncached(hash, opts).then((result) => {
    if (!result.ok) cache.delete(key); // only immutable successes stay cached
    return result;
  });
  cache.set(key, pending);
  return pending;
}

async function runReportUncached(hash: string, opts: ReportOptions): Promise<ReportResult> {
  if (!TX_HASH_RE.test(hash)) {
    return { ok: false, kind: "notFound", message: "not a transaction hash" };
  }
  const url = rpcUrl();
  if (!url) {
    return { ok: false, kind: "error", message: "ETH_RPC_URL is not set" };
  }
  const bin = resolveTracerBin();
  const args = ["report", hash, "--rpc-url", url, "--compact"];
  if (opts.deep) args.push("--deep");
  const backend = backendArg();
  if (backend) args.push("--backend", backend);

  try {
    const { stdout } = await execFileAsync(bin, args, {
      timeout: 180_000,
      maxBuffer: 512 * 1024 * 1024,
    });
    return { ok: true, report: JSON.parse(stdout) as TraceReport };
  } catch (err) {
    // execFile rejections carry a numeric exit code, or a string errno
    // (e.g. "ENOENT") when the spawn itself failed.
    const e = err as { code?: number | string; stderr?: string; killed?: boolean };
    if (e.code === "ENOENT") {
      return {
        ok: false,
        kind: "error",
        message:
          `tracer binary not found (looked for TRACER_BIN, ../target/{release,debug}/tracer, PATH). ` +
          `Build it with: cargo build --release -p tracer-cli`,
      };
    }
    if (e.killed) {
      return { ok: false, kind: "error", message: "tracer timed out after 180s" };
    }
    const stderr = (e.stderr ?? "").trim();
    const lastLine = stderr.split("\n").filter(Boolean).pop() ?? String(e);
    if (e.code === 2) {
      return { ok: false, kind: "notFound", message: lastLine };
    }
    return { ok: false, kind: "error", message: lastLine };
  }
}
