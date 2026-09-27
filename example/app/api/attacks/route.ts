import { getAttackPage, isInvalidCursor } from "@/lib/attacks/feed";

/** `GET /api/attacks?cursor=<c>` — the next page of the home-page attack feed. */
export async function GET(request: Request) {
  const cursor = new URL(request.url).searchParams.get("cursor") || null;
  try {
    const page = await getAttackPage(cursor);
    return Response.json(page, {
      headers: { "Cache-Control": "public, s-maxage=3600, stale-while-revalidate=86400" },
    });
  } catch (err) {
    const noStore = { "Cache-Control": "no-store" };
    if (isInvalidCursor(err)) {
      return Response.json({ error: (err as Error).message }, { status: 400, headers: noStore });
    }
    console.error("attack feed:", err);
    return Response.json({ error: "attack feed unavailable" }, { status: 502, headers: noStore });
  }
}
