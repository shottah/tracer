import Link from "next/link";
import { HashCrumb } from "@/components/hash-form";

export function Shell({
  hash,
  chain,
  children,
}: {
  hash: string;
  chain?: string;
  children: React.ReactNode;
}) {
  return (
    <main className="mx-auto w-full max-w-[1440px] flex-1 px-5 py-5">
      <nav className="mb-4 flex items-center gap-3 text-[13px]">
        <Link href="/" className="font-mono font-semibold text-ink hover:text-accent">
          tracer<span className="text-accent">_</span>
        </Link>
        <span className="text-faint">/</span>
        <HashCrumb hash={hash} chain={chain} />
      </nav>
      {children}
    </main>
  );
}

export function Panel({
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

export function RpcNotConfigured({ hash }: { hash: string }) {
  return (
    <Shell hash={hash}>
      <Panel tone="warn" title="No RPC is configured">
        <p>
          Set <code className="text-warn">DRPC_API_KEY</code> (every supported chain) or{" "}
          <code className="text-warn">ETH_RPC_URL</code> (a single chain) in{" "}
          <code className="text-warn">example/.env.local</code> (see{" "}
          <code className="text-warn">.env.example</code>) and restart the dev server.
        </p>
      </Panel>
    </Shell>
  );
}
