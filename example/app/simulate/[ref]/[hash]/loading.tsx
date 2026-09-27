export default function Loading() {
  return (
    <main className="mx-auto w-full max-w-[1440px] flex-1 px-5 py-5">
      <div className="mb-4 h-5 w-64 animate-pulse rounded bg-panel-2" />
      <div className="rounded-lg border border-hairline bg-panel px-5 py-4">
        <div className="flex items-center gap-3">
          <span className="h-3 w-3 animate-ping rounded-full bg-accent/70" />
          <span className="font-mono text-[13px] text-dim">
            tracing transaction… anvil fork replay can take ~10–30s
          </span>
        </div>
        <div className="mt-4 space-y-2">
          {[88, 72, 94, 60, 80].map((w, i) => (
            <div
              key={i}
              className="h-3.5 animate-pulse rounded bg-panel-2"
              style={{ width: `${w}%`, animationDelay: `${i * 120}ms` }}
            />
          ))}
        </div>
      </div>
    </main>
  );
}
