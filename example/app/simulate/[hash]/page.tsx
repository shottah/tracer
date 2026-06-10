import Link from "next/link";
import { notFound } from "next/navigation";
import { InspectorTabs } from "@/components/inspector-tabs";
import { TxHeader } from "@/components/tx-header";
import { applyLabelOverrides, labelOverrides } from "@/lib/labels";
import { TX_HASH_RE, deepDefault, rpcUrl, runReport } from "@/lib/tracer";

export default async function SimulatePage({
  params,
  searchParams,
}: {
  params: Promise<{ hash: string }>;
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const { hash } = await params;
  const sp = await searchParams;
  if (!TX_HASH_RE.test(hash)) notFound();

  if (!rpcUrl()) {
    return (
      <Shell hash={hash}>
        <Panel tone="warn" title="ETH_RPC_URL is not configured">
          <p>
            Set <code className="text-warn">ETH_RPC_URL</code> in{" "}
            <code className="text-warn">example/.env.local</code> (see{" "}
            <code className="text-warn">.env.example</code>) and restart the dev server.
          </p>
        </Panel>
      </Shell>
    );
  }

  const deepParam = sp.deep;
  const deep = deepParam === "1" ? true : deepParam === "0" ? false : deepDefault();
  const result = await runReport(hash, { deep });

  if (!result.ok) {
    return (
      <Shell hash={hash}>
        <Panel
          tone={result.kind === "notFound" ? "dim" : "neg"}
          title={
            result.kind === "notFound"
              ? "Transaction not found on this endpoint"
              : "Tracing failed"
          }
        >
          <p className="font-mono text-[12.5px] break-all">{result.message}</p>
          {result.kind === "notFound" && (
            <p className="mt-2 text-dim">
              The hash is well-formed but the configured RPC doesn&apos;t know it — wrong
              chain, or the transaction is not mined yet.
            </p>
          )}
        </Panel>
      </Shell>
    );
  }

  // Local labels.json overrides for unverified contracts/wallets.
  const report = applyLabelOverrides(result.report, labelOverrides());

  return (
    <Shell hash={hash}>
      <TxHeader report={report} />
      <InspectorTabs report={report} />
    </Shell>
  );
}

function Shell({ hash, children }: { hash: string; children: React.ReactNode }) {
  return (
    <main className="mx-auto w-full max-w-[1440px] flex-1 px-5 py-5">
      <nav className="mb-4 flex items-center gap-3 text-[13px]">
        <Link href="/" className="font-mono font-semibold text-ink hover:text-accent">
          tracer<span className="text-accent">_</span>
        </Link>
        <span className="text-faint">/</span>
        <span className="truncate font-mono text-[12px] text-dim">simulate/{hash}</span>
      </nav>
      {children}
    </main>
  );
}

function Panel({
  tone,
  title,
  children,
}: {
  tone: "warn" | "neg" | "dim";
  title: string;
  children: React.ReactNode;
}) {
  const border =
    tone === "warn" ? "border-warn/40" : tone === "neg" ? "border-neg/40" : "border-hairline-2";
  return (
    <div className={`rounded-lg border ${border} bg-panel px-5 py-4`}>
      <h2 className="mb-2 text-[15px] font-medium text-ink">{title}</h2>
      <div className="text-[13px] leading-relaxed text-dim">{children}</div>
    </div>
  );
}
