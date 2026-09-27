import { formatLossUsd } from "@/lib/attacks/format";
import { findAttack } from "@/lib/attacks/hall-of-fame";
import { chainBySlug } from "@/lib/chains";
import { shortHex } from "@/lib/format";
import { OG_SIZE, ogCard } from "@/lib/og";

export const alt = "tracer transaction trace";
export const size = OG_SIZE;
export const contentType = "image/png";

/** Built from the URL alone: a share-card fetch must never trigger a trace. */
export default async function Image({
  params,
}: {
  params: Promise<{ ref: string; hash: string }>;
}) {
  const { ref: slug, hash } = await params;
  const chain = chainBySlug(slug)?.name ?? slug;
  const attack = findAttack(slug, hash);
  if (attack) {
    return ogCard({
      eyebrow: "Hall of fame",
      title: `${attack.protocol} exploit`,
      chips: [
        { text: `${formatLossUsd(attack.lossUsd)} lost`, tone: "neg" },
        { text: attack.classification, tone: "violet" },
        { text: `${chain} · ${attack.date}` },
      ],
      subtitle: attack.summary,
    });
  }
  return ogCard({
    eyebrow: "Transaction trace",
    title: `${chain} transaction`,
    chips: [{ text: shortHex(hash, 26) }],
    subtitle: "Decoded call tree, balance changes and fund flow.",
  });
}
