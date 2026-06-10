/**
 * Fetch a prebuilt `tracer` binary from GitHub Releases into ./bin/tracer.
 *
 * Used by the Vercel build (see vercel.json): functions there run on x86_64
 * Amazon Linux, so the fully static musl asset is downloaded and bundled
 * with the serverless function via `outputFileTracingIncludes`.
 *
 * Local dev machines skip this (the app prefers ../target/{release,debug});
 * force with TRACER_FETCH_FORCE=1 and optionally TRACER_TARGET/TRACER_VERSION.
 */

import { execSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync } from "node:fs";

const version = process.env.TRACER_VERSION ?? "v0.1.1";
const target =
  process.env.TRACER_TARGET ??
  (process.platform === "linux" ? "x86_64-unknown-linux-musl" : null);
const dest = "bin/tracer";

if (existsSync(dest)) {
  console.log(`[fetch-tracer] ${dest} already present, skipping`);
  process.exit(0);
}
if (!target || (process.platform !== "linux" && !process.env.TRACER_FETCH_FORCE)) {
  console.log("[fetch-tracer] not a linux build machine; skipping (local dev uses ../target)");
  process.exit(0);
}

const name = `tracer-${version}-${target}`;
const url = `https://github.com/shottah/tracer/releases/download/${version}/${name}.tar.gz`;
console.log(`[fetch-tracer] downloading ${url}`);
mkdirSync("bin", { recursive: true });
execSync(
  `curl -sSfL ${url} -o /tmp/tracer-fetch.tgz && tar -xzf /tmp/tracer-fetch.tgz -C /tmp && cp /tmp/${name}/tracer ${dest}`,
  { stdio: "inherit" },
);
chmodSync(dest, 0o755);
const out = execSync(`./${dest} --version`).toString().trim();
console.log(`[fetch-tracer] installed ${out} -> ${dest}`);
