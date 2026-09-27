"use client";

import { useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import { ChainIcon } from "./chain-icon";

const HASH_RE = /^0x[0-9a-fA-F]{64}$/;

type ChainOption = { slug: string; name: string; testnet: boolean };

/**
 * Home-page hash input. With more than one chain configured, a selector picks
 * the chain; "Auto" searches all of them (`/simulate/<hash>`). On mobile the
 * picker and input share a row and the action gets its own full-width row.
 */
export function HashForm({ chains }: { chains: ChainOption[] }) {
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
      className="flex w-full max-w-2xl flex-col gap-2 sm:flex-row sm:items-center"
    >
      <div className="flex min-w-0 flex-1 items-center gap-2">
        {chains.length > 1 && <ChainPicker chains={chains} value={chain} onChange={setChain} />}
        {/* 16px on mobile: iOS zooms the page when focusing smaller inputs. */}
        <input
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder="0x… transaction hash"
          aria-label="Transaction hash"
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          autoFocus
          className="h-12 min-w-0 flex-1 rounded-md border border-hairline-2 bg-panel px-3.5 font-mono text-[16px] text-ink shadow-[0_0_0_1px_color-mix(in_srgb,var(--accent)_12%,transparent)] placeholder:text-faint outline-none transition-colors focus:border-accent/70 sm:h-11 sm:text-[13px] sm:shadow-none"
        />
      </div>
      <button
        type="submit"
        disabled={!valid || pending}
        className="h-12 shrink-0 cursor-pointer rounded-md bg-accent px-5 text-[14px] font-semibold text-bg transition-[background-color,opacity] hover:bg-[color-mix(in_srgb,var(--accent)_85%,white)] disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-accent sm:h-11 sm:text-[13px]"
      >
        {pending ? "Tracing…" : "Inspect"}
      </button>
    </form>
  );
}

/**
 * Icon-only chain selector: the closed trigger shows just the chain mark;
 * names appear once the listbox is open. Native `<select>` can't render
 * icons in its options, hence the hand-rolled listbox.
 */
function ChainPicker({
  chains,
  value,
  onChange,
}: {
  chains: ChainOption[];
  value: string;
  onChange: (slug: string) => void;
}) {
  const options: ChainOption[] = [
    { slug: "", name: "Auto — search all chains", testnet: false },
    ...chains,
  ];
  const selectedIndex = Math.max(
    0,
    options.findIndex((o) => o.slug === value),
  );
  const selected = options[selectedIndex];
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(selectedIndex);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  useEffect(() => {
    if (!open) return;
    listRef.current?.focus();
    const onPointerDown = (e: PointerEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  const choose = (i: number) => {
    onChange(options[i].slug);
    setOpen(false);
    triggerRef.current?.focus();
  };

  return (
    <div ref={rootRef} className="relative shrink-0">
      <button
        ref={triggerRef}
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={`Chain: ${selected.slug ? selected.name : "Auto"}`}
        title={selected.name}
        onClick={() => {
          setActive(selectedIndex);
          setOpen((o) => !o);
        }}
        className="flex h-12 cursor-pointer items-center gap-1.5 rounded-md border border-hairline-2 bg-panel pr-2 pl-3 outline-none transition-colors hover:border-hairline-2 hover:bg-panel-2 focus-visible:border-accent/60 sm:h-11"
      >
        <ChainIcon slug={selected.slug} testnet={selected.testnet} className="size-5" />
        <svg
          viewBox="0 0 12 12"
          className={`size-3 text-faint transition-transform ${open ? "rotate-180" : ""}`}
          aria-hidden
        >
          <path
            d="m3 4.5 3 3 3-3"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </button>
      {open && (
        <ul
          ref={listRef}
          role="listbox"
          tabIndex={-1}
          aria-label="Chain"
          aria-activedescendant={`chain-opt-${active}`}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") setActive((i) => (i + 1) % options.length);
            else if (e.key === "ArrowUp")
              setActive((i) => (i - 1 + options.length) % options.length);
            else if (e.key === "Home") setActive(0);
            else if (e.key === "End") setActive(options.length - 1);
            else if (e.key === "Enter" || e.key === " ") choose(active);
            else if (e.key === "Escape") {
              setOpen(false);
              triggerRef.current?.focus();
            } else if (e.key === "Tab") setOpen(false);
            else return;
            if (e.key !== "Tab") e.preventDefault();
          }}
          className="absolute top-full left-0 z-30 mt-1.5 min-w-60 rounded-md border border-hairline-2 bg-panel p-1 shadow-xl shadow-black/40 outline-none"
        >
          {options.map((o, i) => (
            <li
              key={o.slug || "auto"}
              id={`chain-opt-${i}`}
              role="option"
              aria-selected={i === selectedIndex}
              onPointerEnter={() => setActive(i)}
              onClick={() => choose(i)}
              className={`flex cursor-pointer items-center gap-2.5 rounded px-2.5 py-2 text-[13px] ${
                i === active ? "bg-panel-2 text-ink" : "text-dim"
              }`}
            >
              <ChainIcon slug={o.slug} testnet={o.testnet} className="size-5" />
              <span className="flex-1">{o.name}</span>
              {o.testnet && <span className="font-mono text-[10.5px] text-warn/80">testnet</span>}
              {i === selectedIndex && <span className="text-accent">✓</span>}
            </li>
          ))}
        </ul>
      )}
    </div>
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
