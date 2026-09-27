/**
 * Shared 1200×630 share card for `opengraph-image.tsx` routes, in the site's
 * dark palette. Satori subset: every multi-child <div> needs display:flex.
 */

import { ImageResponse } from "next/og";

export const OG_SIZE = { width: 1200, height: 630 };

const C = {
  bg: "#0a0d10",
  panel: "#0f141a",
  hairline: "#2a3340",
  ink: "#d7dee6",
  dim: "#7b8794",
  accent: "#5b9cf6",
  neg: "#f47171",
  violet: "#a78bfa",
};

export type OgChip = { text: string; tone?: "neg" | "violet" | "dim" };

export function ogCard({
  eyebrow,
  title,
  subtitle,
  chips = [],
}: {
  eyebrow: string;
  title: string;
  subtitle?: string;
  chips?: OgChip[];
}): ImageResponse {
  const tone = (t: OgChip["tone"]) => (t === "neg" ? C.neg : t === "violet" ? C.violet : C.dim);
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          padding: 64,
          background: C.bg,
          color: C.ink,
          fontFamily: "sans-serif",
        }}
      >
        <div style={{ display: "flex", alignItems: "baseline", fontSize: 40, fontWeight: 700 }}>
          tracer<span style={{ color: C.accent }}>_</span>
          <span style={{ marginLeft: 24, fontSize: 26, fontWeight: 400, color: C.dim }}>
            {eyebrow}
          </span>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
          <div style={{ display: "flex", fontSize: 64, fontWeight: 700, lineHeight: 1.1 }}>
            {title}
          </div>
          {chips.length > 0 && (
            <div style={{ display: "flex", gap: 14 }}>
              {chips.map((c) => (
                <div
                  key={c.text}
                  style={{
                    display: "flex",
                    padding: "6px 18px",
                    borderRadius: 10,
                    border: `2px solid ${tone(c.tone)}`,
                    color: tone(c.tone),
                    fontSize: 28,
                  }}
                >
                  {c.text}
                </div>
              ))}
            </div>
          )}
          {subtitle && (
            <div style={{ display: "flex", fontSize: 30, lineHeight: 1.35, color: C.dim }}>
              {subtitle}
            </div>
          )}
        </div>
        <div
          style={{
            display: "flex",
            borderTop: `2px solid ${C.hairline}`,
            paddingTop: 22,
            fontSize: 24,
            color: C.dim,
          }}
        >
          Invocation flow · Balance changes · Fund flow
        </div>
      </div>
    ),
    OG_SIZE,
  );
}
