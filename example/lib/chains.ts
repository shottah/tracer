/**
 * Chains the inspector knows by name. `slug` is both the URL segment
 * (`/simulate/<slug>/<hash>`) and dRPC's network name
 * (`https://lb.drpc.live/<slug>/<key>`).
 */

export interface Chain {
  slug: string;
  name: string;
  id: number;
  testnet: boolean;
}

export const CHAINS: readonly Chain[] = [
  { slug: "ethereum", name: "Ethereum", id: 1, testnet: false },
  { slug: "base", name: "Base", id: 8453, testnet: false },
  { slug: "arbitrum", name: "Arbitrum One", id: 42161, testnet: false },
  { slug: "sepolia", name: "Sepolia", id: 11155111, testnet: true },
  { slug: "base-sepolia", name: "Base Sepolia", id: 84532, testnet: true },
  { slug: "arbitrum-sepolia", name: "Arbitrum Sepolia", id: 421614, testnet: true },
];

export function chainBySlug(slug: string): Chain | undefined {
  return CHAINS.find((c) => c.slug === slug);
}

/** Known chain for `id`, or a placeholder for endpoints outside the table (e.g. anvil). */
export function chainById(id: number): Chain {
  return (
    CHAINS.find((c) => c.id === id) ?? {
      slug: `chain-${id}`,
      name: `Chain ${id}`,
      id,
      testnet: false,
    }
  );
}
