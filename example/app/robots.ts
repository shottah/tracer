import type { MetadataRoute } from "next";
import { HALL_OF_FAME, attackPath } from "@/lib/attacks/hall-of-fame";
import { siteUrl } from "@/lib/site";

/**
 * Search crawlers get the home page and the curated attack traces only:
 * every other `/simulate/` URL would run a paid debug trace per crawl and
 * is noindex anyway. Link-preview bots may fetch any trace so shared links
 * still unfurl with a card.
 */
const PREVIEW_BOTS = [
  "Twitterbot",
  "facebookexternalhit",
  "LinkedInBot",
  "Slackbot",
  "Slackbot-LinkExpanding",
  "Discordbot",
  "TelegramBot",
  "WhatsApp",
];

export default function robots(): MetadataRoute.Robots {
  return {
    rules: [
      {
        userAgent: "*",
        allow: ["/", ...HALL_OF_FAME.map(attackPath)],
        disallow: ["/simulate/", "/api/"],
      },
      { userAgent: PREVIEW_BOTS, allow: "/", disallow: "/api/" },
    ],
    sitemap: `${siteUrl()}/sitemap.xml`,
  };
}
