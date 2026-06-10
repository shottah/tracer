import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Bundle the tracer binary (downloaded by scripts/fetch-tracer.mjs during
  // the Vercel build) and the optional labels file with the serverless
  // functions — they're read via fs/child_process, invisible to the bundler.
  outputFileTracingIncludes: {
    "/simulate/*": ["./bin/**/*", "./labels.json"],
    "/": ["./bin/**/*"],
  },
};

export default nextConfig;
