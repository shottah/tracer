/**
 * Simplified chain marks, keyed by slug family (`base-sepolia` → `base`).
 * Testnets get a small amber dot so they stay distinguishable at icon size.
 * `slug === ""` is the "Auto" choice: search every chain.
 */
export function ChainIcon({
  slug,
  testnet = false,
  className = "size-5",
}: {
  slug: string;
  testnet?: boolean;
  className?: string;
}) {
  const family = slug.split("-")[0];
  return (
    <span className={`relative inline-flex shrink-0 ${className}`} aria-hidden>
      <svg viewBox="0 0 24 24" className="size-full">
        {family === "ethereum" || family === "sepolia" ? (
          <>
            <circle cx="12" cy="12" r="12" fill="#627eea" />
            <path d="M12 3.5v6.3l5.3 2.4z" fill="#fff" fillOpacity=".6" />
            <path d="M12 3.5 6.7 12.2 12 9.8z" fill="#fff" />
            <path d="M12 16.3v4.2l5.3-7.4z" fill="#fff" fillOpacity=".6" />
            <path d="M12 20.5v-4.2l-5.3-3.2z" fill="#fff" />
            <path d="m12 15.3 5.3-3.1L12 9.8z" fill="#fff" fillOpacity=".2" />
            <path d="m6.7 12.2 5.3 3.1V9.8z" fill="#fff" fillOpacity=".6" />
          </>
        ) : family === "base" ? (
          <>
            <path
              d="M12 0a12 12 0 1 1 0 24 12 12 0 1 1 0-24zM0 10.9h14v2.2H0z"
              fill="#0052ff"
              fillRule="evenodd"
            />
          </>
        ) : family === "arbitrum" ? (
          <>
            <path d="M12 1.5 21.1 6.75v10.5L12 22.5l-9.1-5.25V6.75z" fill="#213147" />
            <path
              d="m13.2 8.2 4.2 7.1-1.9 1.1-3.5-6zM10.2 8.2l1.1 1.9-3.4 6.3-1.9-1.1z"
              fill="#12aaff"
            />
            <path
              d="M12 1.5 21.1 6.75v10.5L12 22.5l-9.1-5.25V6.75z"
              fill="none"
              stroke="#9dcced"
              strokeWidth="1"
            />
          </>
        ) : slug === "" ? (
          <g fill="none" stroke="var(--accent)" strokeWidth="1.6">
            <circle cx="12" cy="12" r="9.5" />
            <ellipse cx="12" cy="12" rx="4" ry="9.5" />
            <path d="M2.5 12h19" />
          </g>
        ) : (
          <circle cx="12" cy="12" r="11" fill="none" stroke="var(--dim)" strokeWidth="1.6" />
        )}
      </svg>
      {testnet && (
        <span className="absolute -right-0.5 -bottom-0.5 size-2 rounded-full bg-warn ring-2 ring-panel" />
      )}
    </span>
  );
}
