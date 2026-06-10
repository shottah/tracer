"use client";

/** Small shared primitives: chips, address tags, copy affordances. */

import { useState } from "react";
import { shortAddr } from "@/lib/format";

export function Chip({
  children,
  tone = "dim",
  className = "",
  title,
}: {
  children: React.ReactNode;
  tone?: "dim" | "pos" | "neg" | "accent" | "warn" | "violet" | "cyan";
  className?: string;
  title?: string;
}) {
  const tones: Record<string, string> = {
    dim: "border-hairline-2 text-dim",
    pos: "border-pos/40 text-pos",
    neg: "border-neg/40 text-neg",
    accent: "border-accent/40 text-accent",
    warn: "border-warn/40 text-warn",
    violet: "border-violet/40 text-violet",
    cyan: "border-cyan/40 text-cyan",
  };
  return (
    <span
      title={title}
      className={`inline-flex shrink-0 items-center rounded border px-1.5 py-px font-mono text-[10px] leading-4 tracking-wide ${tones[tone]} ${className}`}
    >
      {children}
    </span>
  );
}

export function CopyButton({ text, className = "" }: { text: string; className?: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      aria-label="copy"
      title={copied ? "copied" : "copy"}
      onClick={() => {
        navigator.clipboard.writeText(text).then(() => {
          setCopied(true);
          setTimeout(() => setCopied(false), 1200);
        });
      }}
      className={`cursor-pointer text-faint transition-colors hover:text-ink ${className}`}
    >
      {copied ? (
        <span className="text-pos text-[11px] font-mono">✓</span>
      ) : (
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4">
          <rect x="9" y="9" width="12" height="12" rx="2" />
          <path d="M5 15V5a2 2 0 0 1 2-2h10" />
        </svg>
      )}
    </button>
  );
}

/** `[Sender]`-style role badge + label + mono address, Phalcon fashion. */
export function AddressTag({
  address,
  label,
  badge,
  mono = true,
  full = false,
}: {
  address: string;
  label?: string;
  badge?: "sender" | "receiver";
  mono?: boolean;
  full?: boolean;
}) {
  return (
    <span className="inline-flex min-w-0 items-center gap-1.5" title={address}>
      {badge === "sender" && (
        <span className="shrink-0 rounded bg-warn/15 px-1 py-px font-mono text-[10px] text-warn">
          [Sender]
        </span>
      )}
      {badge === "receiver" && (
        <span className="shrink-0 rounded bg-accent/15 px-1 py-px font-mono text-[10px] text-accent">
          [Receiver]
        </span>
      )}
      {label ? (
        <span className="truncate text-ink">{label}</span>
      ) : (
        <span className={`truncate text-ink ${mono ? "font-mono" : ""}`}>
          {full ? address : shortAddr(address)}
        </span>
      )}
      <CopyButton text={address} className="shrink-0" />
    </span>
  );
}

export function Toggle({
  on,
  onChange,
  label,
}: {
  on: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <button
      type="button"
      onClick={() => onChange(!on)}
      className="inline-flex cursor-pointer items-center gap-1.5 text-[12px] text-dim transition-colors hover:text-ink"
    >
      <span
        className={`relative h-3.5 w-6.5 rounded-full border transition-colors ${
          on ? "border-accent/60 bg-accent/30" : "border-hairline-2 bg-panel-2"
        }`}
      >
        <span
          className={`absolute top-[2px] h-2 w-2 rounded-full transition-all ${
            on ? "left-3.5 bg-accent" : "left-[2px] bg-faint"
          }`}
        />
      </span>
      {label}
    </button>
  );
}

export function PanelNote({ children }: { children: React.ReactNode }) {
  return (
    <div className="rounded-md border border-hairline bg-panel px-4 py-3 text-[13px] text-dim">
      {children}
    </div>
  );
}
