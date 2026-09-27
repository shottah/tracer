import { OG_SIZE, ogCard } from "@/lib/og";

export const alt = "tracer: open-source EVM transaction inspector";
export const size = OG_SIZE;
export const contentType = "image/png";

export default function Image() {
  return ogCard({
    eyebrow: "EVM transaction inspector",
    title: "See exactly what a transaction did",
    subtitle:
      "Decoded call trees, token balance changes and fund flow for Ethereum, Base and Arbitrum. Free and open source.",
  });
}
