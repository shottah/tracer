export default function Loading() {
  return (
    <main className="mx-auto w-full max-w-[1440px] flex-1 px-5 py-5">
      <div className="mb-4 h-5 w-64 animate-pulse rounded bg-panel-2" />
      <div className="rounded-lg border border-hairline bg-panel px-5 py-4">
        <div className="flex items-center gap-3">
          <span className="h-3 w-3 animate-ping rounded-full bg-accent/70" />
          <span className="font-mono text-[13px] text-dim">locating transaction…</span>
        </div>
      </div>
    </main>
  );
}
