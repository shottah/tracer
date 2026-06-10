/**
 * Monkey-patch labels: a flat `labels.json` next to the app (or at
 * `LABELS_FILE`) mapping lowercase addresses to display names, for contracts
 * that aren't ABI-verified on-chain:
 *
 *   { "0x1a9ad597…": "VFE", "0x04a929e2…": "USD Curve Pool" }
 *
 * Overrides are applied to a copy of the (cached, immutable) report after it
 * is produced, so editing the file takes effect on the next page load — and
 * they win over any label tracer derived (built-ins, token symbols).
 */

import { existsSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import type { TraceReport } from "./types";

const ADDRESS_RE = /^0x[0-9a-fA-F]{40}$/;

let cached: { file: string; mtimeMs: number; map: Record<string, string> } | null = null;
let envCached: { raw: string; map: Record<string, string> } | null = null;

function sanitize(raw: unknown, origin: string): Record<string, string> {
  const map: Record<string, string> = {};
  if (raw && typeof raw === "object") {
    for (const [key, value] of Object.entries(raw)) {
      if (ADDRESS_RE.test(key) && typeof value === "string" && value.trim()) {
        map[key.toLowerCase()] = value.trim();
      }
    }
  } else if (raw !== undefined) {
    console.warn(`${origin}: expected an object of address → label`);
  }
  return map;
}

/**
 * Load the override map (lowercase address → label): `labels.json` on disk,
 * overlaid with the `LABELS_JSON` env var (inline JSON — handy on serverless
 * platforms where shipping an extra file is awkward). Env entries win.
 */
export function labelOverrides(): Record<string, string> {
  const fromFile = fileOverrides();
  const fromEnv = envOverrides();
  return { ...fromFile, ...fromEnv };
}

function fileOverrides(): Record<string, string> {
  const file = process.env.LABELS_FILE ?? path.join(process.cwd(), "labels.json");
  try {
    if (!existsSync(file)) return {};
    const mtimeMs = statSync(file).mtimeMs;
    if (cached && cached.file === file && cached.mtimeMs === mtimeMs) return cached.map;
    const map = sanitize(JSON.parse(readFileSync(file, "utf8")), file);
    cached = { file, mtimeMs, map };
    return map;
  } catch (err) {
    console.warn(`labels.json ignored (${file}):`, err);
    return {};
  }
}

function envOverrides(): Record<string, string> {
  const raw = process.env.LABELS_JSON;
  if (!raw) return {};
  if (envCached && envCached.raw === raw) return envCached.map;
  try {
    const map = sanitize(JSON.parse(raw), "LABELS_JSON");
    envCached = { raw, map };
    return map;
  } catch (err) {
    console.warn("LABELS_JSON ignored:", err);
    return {};
  }
}

/**
 * Overlay labels onto a report copy: the shared `addressLabels` dictionary
 * plus the per-row labels the views read directly (balance-change rows,
 * fund-flow nodes). Overrides replace any existing label for the same
 * address regardless of key casing.
 */
export function applyLabelOverrides(
  report: TraceReport,
  overrides: Record<string, string>,
): TraceReport {
  if (Object.keys(overrides).length === 0) return report;
  const get = (addr: string) => overrides[addr.toLowerCase()];

  const addressLabels: Record<string, string> = {};
  for (const [k, v] of Object.entries(report.addressLabels ?? {})) {
    addressLabels[k.toLowerCase()] = v;
  }
  Object.assign(addressLabels, overrides);

  return {
    ...report,
    addressLabels,
    balanceChanges: report.balanceChanges && {
      ...report.balanceChanges,
      changes: report.balanceChanges.changes.map((c) =>
        get(c.address) ? { ...c, label: get(c.address) } : c,
      ),
    },
    fundFlow: report.fundFlow && {
      ...report.fundFlow,
      nodes: report.fundFlow.nodes.map((n) => (get(n.id) ? { ...n, label: get(n.id) } : n)),
    },
  };
}
