import { NextResponse, type NextRequest } from "next/server";
import { siteUrl } from "@/lib/site";

/**
 * Permanently redirects the App Hosting default domain (`*.hosted.app`) to
 * the canonical SITE_URL, so search engines see one host. Any other host
 * (the custom domain, localhost, forks) passes through untouched.
 */
export function proxy(request: NextRequest) {
  const host = (request.headers.get("x-forwarded-host") ?? request.headers.get("host") ?? "")
    .split(",")[0]
    .trim()
    .toLowerCase();
  const target = siteUrl();
  if (!host.endsWith(".hosted.app") || new URL(target).host === host) return;
  const { pathname, search } = request.nextUrl;
  return NextResponse.redirect(`${target}${pathname}${search}`, 308);
}

export const config = {
  matcher: ["/((?!_next/static|_next/image).*)"],
};
