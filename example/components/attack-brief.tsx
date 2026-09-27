import { Chip } from "@/components/ui";
import { formatLossUsd } from "@/lib/attacks/format";
import type { Attack } from "@/lib/attacks/types";
import { chainBySlug } from "@/lib/chains";

/**
 * Server-rendered context above a hall-of-fame trace. It's the page's crawlable
 * text (the inspector below is mostly canvas and client state) and its h1.
 */
export function AttackBrief({ attack: a }: { attack: Attack }) {
  const chain = chainBySlug(a.chain)?.name ?? a.chain;
  return (
    <section className="rise mb-4 rounded-lg border border-hairline bg-panel px-5 py-4">
      <h1 className="text-[18px] font-semibold text-ink">{a.protocol} exploit transaction</h1>
      <div className="mt-2 flex flex-wrap items-center gap-1.5">
        <Chip tone="neg">{formatLossUsd(a.lossUsd)} lost</Chip>
        <Chip>{chain}</Chip>
        <Chip tone="violet">{a.classification}</Chip>
        <Chip>
          <time dateTime={a.date}>{a.date}</time>
        </Chip>
      </div>
      <p className="mt-2.5 max-w-3xl text-[13.5px] leading-relaxed text-dim">{a.summary}</p>
      <p className="mt-1.5 max-w-3xl text-[12.5px] leading-relaxed text-faint">
        Below is the attack transaction replayed on {chain}: the decoded invocation flow (every
        call, event and revert), per-account balance changes, and the fund flow between the
        attacker and the protocol.
      </p>
    </section>
  );
}
