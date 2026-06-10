import Link from "next/link";

export default function NotFound() {
  return (
    <main className="dotgrid flex min-h-screen flex-col items-center justify-center gap-4 px-6">
      <h1 className="font-mono text-3xl text-ink">
        404<span className="text-neg">_</span>
      </h1>
      <p className="text-[14px] text-dim">
        That doesn&apos;t look like a transaction hash (expected <code>0x</code> + 64 hex chars).
      </p>
      <Link
        href="/"
        className="rounded-md border border-accent/50 bg-accent/15 px-4 py-2 text-[13px] text-accent hover:bg-accent/25"
      >
        ← back to tracer_
      </Link>
    </main>
  );
}
