"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";

const HASH_RE = /^0x[0-9a-fA-F]{64}$/;

export function HashForm() {
  const router = useRouter();
  const [value, setValue] = useState("");
  const [pending, setPending] = useState(false);
  const valid = HASH_RE.test(value.trim());

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        if (!valid) return;
        setPending(true);
        router.push(`/simulate/${value.trim()}`);
      }}
      className="flex w-full max-w-2xl items-center gap-2"
    >
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
