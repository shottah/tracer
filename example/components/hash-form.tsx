"use client";

import { useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";

const HASH_RE = /^0x[0-9a-fA-F]{64}$/;

/**
 * Home-page hash input. With more than one chain configured, a selector picks
 * the chain; "Auto" searches all of them (`/simulate/<hash>`).
 */
export function HashForm({ chains }: { chains: { slug: string; name: string }[] }) {
  const router = useRouter();
  const [value, setValue] = useState("");
  const [chain, setChain] = useState("");
  const [pending, setPending] = useState(false);
  const valid = HASH_RE.test(value.trim());

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        if (!valid) return;
        setPending(true);
        router.push(chain ? `/simulate/${chain}/${value.trim()}` : `/simulate/${value.trim()}`);
      }}
      className="flex w-full max-w-2xl items-center gap-2"
    >
      {chains.length > 1 && (
        <select
          value={chain}
          onChange={(e) => setChain(e.target.value)}
          aria-label="Chain"
          className="h-11 shrink-0 cursor-pointer rounded-md border border-hairline-2 bg-panel px-2.5 font-mono text-[13px] text-ink outline-none transition-colors focus:border-accent/60"
        >
          <option value="">Auto</option>
          {chains.map((c) => (
            <option key={c.slug} value={c.slug}>
              {c.name}
            </option>
          ))}
        </select>
      )}
      <input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder="0x… transaction hash"
        spellCheck={false}
        autoFocus
        className="h-11 min-w-0 flex-1 rounded-md border border-hairline-2 bg-panel px-3.5 font-mono text-[13px] text-ink placeholder:text-faint outline-none transition-colors focus:border-accent/60"
      />
      <button
        type="submit"
        disabled={!valid || pending}
        className="h-11 shrink-0 cursor-pointer rounded-md border border-accent/50 bg-accent/15 px-5 text-[13px] font-medium text-accent transition-colors hover:bg-accent/25 disabled:cursor-not-allowed disabled:opacity-40"
      >
        {pending ? "Tracing…" : "Inspect"}
      </button>
    </form>
  );
}

/**
 * Breadcrumb hash on the simulate page; click to paste another hash.
 * The button and input share one box (width, height, padding, border) so
 * toggling between them doesn't shift the layout or the text.
 */
const CRUMB_BOX =
  "box-content h-[18px] w-[66ch] min-w-0 rounded border px-1 font-mono text-[12px] leading-[18px]";

export function HashCrumb({ hash, chain }: { hash: string; chain?: string }) {
  const router = useRouter();
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState(hash);
  const inputRef = useRef<HTMLInputElement>(null);
  const next = value.trim();
  const valid = HASH_RE.test(next);

  // Select everything on open so a paste replaces the current hash.
  useEffect(() => {
    if (editing) inputRef.current?.select();
  }, [editing]);

  const cancel = () => {
    setValue(hash);
    setEditing(false);
  };

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        if (!valid) return;
        setEditing(false);
        // A new hash may live on another chain: let the locator find it.
        if (next.toLowerCase() !== hash.toLowerCase()) router.push(`/simulate/${next}`);
      }}
      className="flex min-w-0 items-center font-mono text-[12px] leading-[18px]"
    >
      <span className="text-dim">simulate/{chain && `${chain}/`}</span>
      {editing ? (
        <input
          ref={inputRef}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          autoFocus
          onKeyDown={(e) => e.key === "Escape" && cancel()}
          // Window blur (switching apps to copy a hash) shouldn't close the editor.
          onBlur={() => document.hasFocus() && cancel()}
          placeholder="0x… transaction hash"
          spellCheck={false}
          aria-label="Transaction hash"
          aria-invalid={!valid}
          className={`${CRUMB_BOX} bg-panel-2 text-ink placeholder:text-faint outline-none ${
            valid ? "border-accent/60" : "border-neg/60"
          }`}
        />
      ) : (
        <button
          type="button"
          title="Click to inspect another transaction"
          onClick={() => {
            setValue(hash);
            setEditing(true);
          }}
          className={`${CRUMB_BOX} cursor-text truncate border-transparent text-left text-dim transition-colors hover:bg-panel-2 hover:text-ink`}
        >
          {hash}
        </button>
      )}
    </form>
  );
}
