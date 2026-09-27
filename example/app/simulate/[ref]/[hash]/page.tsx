import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { AttackBrief } from "@/components/attack-brief";
import { InspectorTabs } from "@/components/inspector-tabs";
import { RememberTx } from "@/components/remember-tx";
import { Panel, RpcNotConfigured, Shell } from "@/components/simulate-shell";
import { TrackEvent } from "@/components/track-event";
import { TxHeader } from "@/components/tx-header";
import { formatLossUsd } from "@/lib/attacks/format";
import { findAttack } from "@/lib/attacks/hall-of-fame";
import { chainBySlug } from "@/lib/chains";
import { endpoints, redactSecrets, type Endpoint } from "@/lib/endpoints";
import { countFrames, displayName, shortHex } from "@/lib/format";
import { applyLabelOverrides, labelOverrides } from "@/lib/labels";
import { TX_HASH_RE, deepDefault, runReport } from "@/lib/tracer";

// Tracing big transactions takes a while; allow up to 5 minutes on Vercel.
export const maxDuration = 300;

/**
 * Never runs the trace: link-preview bots block on metadata, and the page
 * render already pays for one. Only curated attacks are indexable; any other
 * hash is an unbounded, per-crawl-billed URL space (robots.txt blocks it too).
 */
export async function generateMetadata({
  params,
}: {
  params: Promise<{ ref: string; hash: string }>;
}): Promise<Metadata> {
  const { ref: slug, hash } = await params;
  const path = `/simulate/${slug}/${hash}`;
  const attack = findAttack(slug, hash);
  if (attack) {
    const chain = chainBySlug(attack.chain)?.name ?? attack.chain;
    const title = `${attack.protocol} exploit (${formatLossUsd(attack.lossUsd)}): ${chain} transaction trace`;
    const description = `${attack.summary} Replay the ${attack.date} attack: call tree, balance changes and fund flow.`;
    return {
      title,
      description,
      alternates: { canonical: path },
      openGraph: { type: "article", siteName: "tracer", url: path, title, description },
    };
  }
  const chain = chainBySlug(slug)?.name ?? slug;
  const title = `Transaction ${shortHex(hash, 14)} on ${chain}`;
  const description = `Call tree, balance changes and fund flow of ${chain} transaction ${hash}.`;
  return {
    title,
    description,
    alternates: { canonical: path },
    robots: { index: false, follow: true },
    openGraph: { type: "website", siteName: "tracer", url: path, title, description },
  };
}

/** `/simulate/<chain>/<hash>` — trace `hash` on a specific chain. */
export default async function SimulatePage({
  params,
  searchParams,
}: {
  params: Promise<{ ref: string; hash: string }>;
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const { ref: slug, hash } = await params;
  const sp = await searchParams;
  if (!TX_HASH_RE.test(hash)) notFound();
  const attack = findAttack(slug, hash);

  let eps: Endpoint[];
  try {
    eps = await endpoints();
  } catch (err) {
    return (
      <Shell hash={hash} chain={slug}>
        <meta name="robots" content="noindex" />
        <TrackEvent
          event={{ name: "trace_error", params: { chain: slug, error_kind: "rpc_unreachable" } }}
        />
        <Panel tone="neg" title="RPC unreachable">
          <p className="font-mono text-[12.5px] break-all">
            {redactSecrets(String((err as Error).message ?? err))}
          </p>
        </Panel>
      </Shell>
    );
  }
  if (eps.length === 0) return <RpcNotConfigured hash={hash} />;

  const endpoint = eps.find((ep) => ep.chain.slug === slug);
  if (!endpoint) {
    return (
      <Shell hash={hash} chain={slug}>
        <TrackEvent
          event={{ name: "trace_error", params: { chain: slug, error_kind: "chain_unavailable" } }}
        />
        <Panel tone="warn" title={`Chain "${slug}" is not available`}>
          <p>
            This server can trace on:{" "}
            {eps.map((ep, i) => (
              <span key={ep.chain.slug}>
                {i > 0 && ", "}
                <Link
                  href={`/simulate/${ep.chain.slug}/${hash}`}
                  className="text-accent underline-offset-4 hover:underline"
                >
                  {ep.chain.name}
                </Link>
              </span>
            ))}
            .
          </p>
        </Panel>
      </Shell>
    );
  }

  const deepParam = sp.deep;
  const deep = deepParam === "1" ? true : deepParam === "0" ? false : deepDefault();
  const result = await runReport(hash, endpoint, { deep });

  if (!result.ok) {
    return (
      <Shell hash={hash} chain={slug}>
        {/* Streaming already sent 200; keep a transient failure out of the index. */}
        <meta name="robots" content="noindex" />
        {attack && <AttackBrief attack={attack} />}
        <TrackEvent
          event={{
            name: "trace_error",
            params: {
              chain: slug,
              error_kind: result.kind === "notFound" ? "not_found" : "trace_failed",
            },
          }}
        />
        <Panel
          tone={result.kind === "notFound" ? "dim" : "neg"}
          title={
            result.kind === "notFound"
              ? `Transaction not found on ${endpoint.chain.name}`
              : "Tracing failed"
          }
        >
          <p className="font-mono text-[12.5px] break-all">{result.message}</p>
          {result.kind === "notFound" && (
            <p className="mt-2 text-dim">
              The hash is well-formed but {endpoint.chain.name} doesn&apos;t know it — wrong
              chain, or the transaction is not mined yet.{" "}
              {eps.length > 1 && (
                <Link
                  href={`/simulate/${hash}`}
                  className="text-accent underline-offset-4 hover:underline"
                >
                  Search all chains
                </Link>
              )}
            </p>
          )}
        </Panel>
      </Shell>
    );
  }

  // Local labels.json overrides for unverified contracts/wallets.
  const report = applyLabelOverrides(result.report, labelOverrides());
  const { tx } = report;
  const method = !tx.to
    ? "create"
    : tx.input === "0x"
      ? "transfer"
      : (report.trace?.decoded?.name ?? tx.input.slice(0, 10));
  // Unlabeled targets fall back to a short address so feed rows stay distinct.
  const target = tx.to ?? tx.contractCreated;

  return (
    <Shell hash={hash} chain={slug}>
      <RememberTx
        visit={{
          chain: slug,
          hash,
          ok: tx.status,
          method,
          to: target && displayName(report, target),
        }}
      />
      <TrackEvent
        event={{
          name: "trace_view",
          params: {
            chain: slug,
            deep,
            tx_status: tx.status ? "success" : "reverted",
            calls: countFrames(report),
          },
        }}
      />
      {attack && <AttackBrief attack={attack} />}
      <TxHeader report={report} />
      <InspectorTabs report={report} />
    </Shell>
  );
}
