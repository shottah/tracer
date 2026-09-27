import type { Metadata } from "next";
import { GoogleAnalytics } from "@next/third-parties/google";
import { IBM_Plex_Mono, IBM_Plex_Sans } from "next/font/google";
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

export const metadata: Metadata = {
  title: "tracer — EVM transaction inspector",
  description:
    "Open-source Phalcon/Tenderly-style transaction inspection: invocation flow, balance changes, fund flow.",
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
