import type { Metadata } from "next";
import { GoogleAnalytics } from "@next/third-parties/google";
import { IBM_Plex_Mono, IBM_Plex_Sans } from "next/font/google";
import { SITE_DESCRIPTION, SITE_NAME, siteUrl } from "@/lib/site";
import "./globals.css";

const plexSans = IBM_Plex_Sans({
  variable: "--font-plex-sans",
  subsets: ["latin"],
  weight: ["400", "500", "600"],
});

const plexMono = IBM_Plex_Mono({
  variable: "--font-plex-mono",
  subsets: ["latin"],
  weight: ["400", "500", "600"],
});

// Pages override title/description/canonical; openGraph and twitter set here
// are replaced wholesale by any page that defines its own.
export const metadata: Metadata = {
  metadataBase: new URL(siteUrl()),
  title: {
    default: "tracer — EVM transaction inspector",
    template: "%s · tracer",
  },
  description: SITE_DESCRIPTION,
  applicationName: SITE_NAME,
  openGraph: {
    type: "website",
    siteName: SITE_NAME,
    title: "tracer — EVM transaction inspector",
    description: SITE_DESCRIPTION,
  },
  twitter: { card: "summary_large_image" },
};

// Public by design (it ships in the page anyway); inlined at build time.
const gaId = process.env.NEXT_PUBLIC_GA_ID;

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={`${plexSans.variable} ${plexMono.variable} h-full antialiased`}>
      <body className="min-h-full flex flex-col">{children}</body>
      {/* GA4 only when a measurement ID is configured, so local dev and forks stay untracked. */}
      {gaId && <GoogleAnalytics gaId={gaId} />}
    </html>
  );
}
