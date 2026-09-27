import type { MetadataRoute } from "next";
import { HALL_OF_FAME, attackPath } from "@/lib/attacks/hall-of-fame";
import { siteUrl } from "@/lib/site";

/** The home page plus every curated attack trace (see robots.ts). */
export default function sitemap(): MetadataRoute.Sitemap {
  const base = siteUrl();
  return [
    { url: `${base}/`, changeFrequency: "weekly", priority: 1 },
    ...HALL_OF_FAME.map((a) => ({
      url: `${base}${attackPath(a)}`,
      // A mined transaction never changes; its write-up rarely does.
      changeFrequency: "yearly" as const,
      priority: 0.8,
    })),
  ];
}
