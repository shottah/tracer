import Link from "next/link";
import { notFound, redirect } from "next/navigation";
import { Panel, RpcNotConfigured, Shell } from "@/components/simulate-shell";
import { endpoints, locateTx, redactSecrets, type Endpoint } from "@/lib/endpoints";
import { TX_HASH_RE } from "@/lib/tracer";

/**
 * `/simulate/<hash>` — find which chain the transaction is on, then redirect
 * to `/simulate/<chain>/<hash>`. (The segment is `[ref]`, not `[hash]`,
 * because Next requires one param name per level and the nested route
 * uses it for the chain.)
 */
export default async function LocatePage({
  params,
  searchParams,
}: {
  params: Promise<{ ref: string }>;
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const { ref: hash } = await params;
  const sp = await searchParams;
  if (!TX_HASH_RE.test(hash)) notFound();

  let eps: Endpoint[];
  try {
    eps = await endpoints();
  } catch (err) {
    return (
      <Shell hash={hash}>
        <Panel tone="neg" title="RPC unreachable">
          <p className="font-mono text-[12.5px] break-all">
            {redactSecrets(String((err as Error).message ?? err))}
          </p>
        </Panel>
      </Shell>
    );
  }
  if (eps.length === 0) return <RpcNotConfigured hash={hash} />;

  const query = typeof sp.deep === "string" ? `?deep=${encodeURIComponent(sp.deep)}` : "";
  const target = (slug: string) => `/simulate/${slug}/${hash}${query}`;

  // A single endpoint has nothing to choose between; its page reports misses.
  if (eps.length === 1) redirect(target(eps[0].chain.slug));

  const { hits, failed } = await locateTx(hash, eps);
  if (hits.length === 1) redirect(target(hits[0].slug));

  if (hits.length > 1) {
    return (
      <Shell hash={hash}>
        <Panel tone="warn" title="Found on more than one chain">
          <p>Pick the chain to trace:</p>
          <ul className="mt-2 space-y-1">
            {hits.map((c) => (
              <li key={c.slug}>
                <Link
                  href={target(c.slug)}
                  className="text-accent underline-offset-4 hover:underline"
                >
                  {c.name}
                </Link>
              </li>
            ))}
          </ul>
        </Panel>
      </Shell>
    );
  }

  const searched = eps.filter((ep) => !failed.some((f) => f.chain.slug === ep.chain.slug));
  return (
    <Shell hash={hash}>
      <Panel tone={failed.length ? "warn" : "dim"} title="Transaction not found">
        {searched.length > 0 && (
          <p>
            Searched {searched.map((ep) => ep.chain.name).join(", ")} — the hash is well-formed
            but no chain knows it, or it is not mined yet.
          </p>
        )}
        {failed.length > 0 && (
          <>
            <p className={searched.length ? "mt-2" : undefined}>
              These lookups failed, so it may be on one of them:
            </p>
            <ul className="mt-1 space-y-1 font-mono text-[12.5px]">
              {failed.map((f) => (
                <li key={f.chain.slug}>
                  <Link
                    href={target(f.chain.slug)}
                    className="text-accent underline-offset-4 hover:underline"
                  >
                    {f.chain.name}
                  </Link>{" "}
                  <span className="break-all">— {f.message}</span>
                </li>
              ))}
            </ul>
          </>
        )}
      </Panel>
    </Shell>
  );
}
