import { AttackFeed } from "@/components/attack-feed";
import { HashForm } from "@/components/hash-form";
import { RecentFeed } from "@/components/recent-feed";
import { getAttackPage } from "@/lib/attacks/feed";
import type { AttackPage } from "@/lib/attacks/types";
import { CHAINS } from "@/lib/chains";
import { rpcMode, safeHost } from "@/lib/endpoints";
import { RECENT_COOKIE, parseRecent } from "@/lib/recent";
import { resolveTracerBin } from "@/lib/tracer";
import { cookies } from "next/headers";
import { existsSync } from "node:fs";

// Environment status chips must reflect the running server, not build time.
export const dynamic = "force-dynamic";

export default async function Home() {
  const mode = rpcMode();
  const chains =
    mode.kind === "drpc" ? CHAINS.map(({ slug, name, testnet }) => ({ slug, name, testnet })) : [];
  const mainnets = CHAINS.filter((c) => !c.testnet).map((c) => c.slug);
  const hasTestnets = CHAINS.some((c) => c.testnet);
  const bin = resolveTracerBin();
  const binFound = bin === "tracer" ? undefined : existsSync(bin);
  const recent = parseRecent((await cookies()).get(RECENT_COOKIE)?.value);

  // The feed is decoration: if its source fails, omit the section.
  let attacks: AttackPage | null = null;
  try {
    attacks = await getAttackPage(null);
  } catch (err) {
    console.error("attack feed:", err);
  }

  return (
    <main className="dotgrid flex min-h-screen flex-col items-center justify-center py-16 px-6">
      <div className="rise flex w-full max-w-2xl flex-col items-start gap-6">
        <div>
          <h1 className="font-mono text-4xl font-semibold tracking-tight text-ink">
            tracer<span className="text-accent">_</span>
          </h1>
          <p className="mt-2 max-w-xl text-[14px] leading-relaxed text-dim">
            Headless EVM transaction inspection — invocation flow, balance changes, and fund
            flow, Phalcon-style. Paste a transaction hash from{" "}
            {chains.length ? "any supported chain" : "the configured chain"}.
          </p>
        </div>

        <HashForm chains={chains} />

        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-[11.5px] text-faint">
          <span>
            rpc:{" "}
            {mode.kind === "drpc" ? (
              <span className="text-pos">
                drpc · {mainnets.join(", ")}
                {hasTestnets && " and sepolias"}
              </span>
            ) : mode.kind === "url" ? (
              <span className="text-pos">{safeHost(mode.url) ?? "ETH_RPC_URL"}</span>
            ) : (
              <span className="text-neg">DRPC_API_KEY / ETH_RPC_URL not set</span>
            )}
          </span>
          <span>
            engine:{" "}
            {binFound === false ? (
              <span className="text-neg">
                tracer binary missing — cargo build --release -p tracer-cli
              </span>
            ) : (
              <span className="text-pos">tracer</span>
            )}
          </span>
          <a
            className="text-dim underline-offset-4 hover:text-ink hover:underline"
            href="https://github.com/shottah/tracer"
          >
            github.com/shottah/tracer
          </a>
        </div>

        <div className="grid w-full grid-cols-1 gap-2.5 sm:grid-cols-3">
          {[
            ["Invocation Flow", "Decoded call tree with events, reverts, and storage writes"],
            ["Balance Changes", "Exact per-account native + token deltas, gas-aware"],
            ["Fund Flow", "Ordered transfer graph on the canvas — arrow-key navigable"],
          ].map(([title, desc]) => (
            <div key={title} className="rounded-md border border-hairline bg-panel/80 px-3.5 py-3">
              <div className="text-[12.5px] font-medium text-ink">{title}</div>
              <div className="mt-1 text-[11.5px] leading-snug text-dim">{desc}</div>
            </div>
          ))}
        </div>

        <RecentFeed initial={recent} />

        {attacks && attacks.items.length > 0 && (
          <section className="flex w-full flex-col gap-2.5">
            <h2 className="font-mono text-[11.5px] uppercase tracking-wider text-faint">
              Hall of fame — replay a famous attack
            </h2>
            <AttackFeed initial={attacks} />
          </section>
        )}
      </div>
    </main>
  );
}
