//! Built-in labels for well-known addresses, wrapped-native registries, and
//! native-currency symbols. Deliberately small; consumers can layer their own.

use alloy_primitives::{Address, address};

/// Display symbol for a chain's native currency.
pub fn native_symbol(chain_id: u64) -> &'static str {
    match chain_id {
        137 => "POL",
        56 => "BNB",
        43114 => "AVAX",
        100 => "xDAI",
        250 => "FTM",
        // Ethereum, OP-stack chains, Arbitrum, anvil default (31337), ...
        _ => "ETH",
    }
}

/// Canonical wrapped-native token, used to treat `Deposit`/`Withdrawal`
/// events as wrap/unwrap flows without false positives from other contracts.
pub fn wrapped_native(chain_id: u64) -> Option<Address> {
    Some(match chain_id {
        1 => address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"), // WETH
        10 | 8453 => address!("0x4200000000000000000000000000000000000006"), // WETH (OP stack)
        42161 => address!("0x82af49447d8a07e3bd95bd0d56f35241523fbab1"), // WETH (Arbitrum)
        137 => address!("0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270"), // WPOL
        56 => address!("0xbb4cdb9cbd36b01bd1cbaef60af814a3f6f0ee75"), // WBNB
        _ => return None,
    })
}

const MAINNET_LABELS: &[(Address, &str)] = &[
    (address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"), "WETH"),
    (address!("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"), "USDC"),
    (address!("0xdac17f958d2ee523a2206206994597c13d831ec7"), "USDT"),
    (address!("0x6b175474e89094c44da98b954eedeac495271d0f"), "DAI"),
    (address!("0x2260fac5e5542a773aa44fbcfedf7c193bc2c599"), "WBTC"),
    (address!("0xae7ab96520de3a18e5e111b5eaab095312d7fe84"), "stETH"),
    (address!("0x7a250d5630b4cf539739df2c5dacb4c659f2488d"), "Uniswap V2: Router02"),
    (address!("0xe592427a0aece92de3edee1f18e0157c05861564"), "Uniswap V3: Router"),
    (address!("0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45"), "Uniswap V3: Router02"),
    (address!("0x3fc91a3afd70395cd496c647d5a6cc9d4b2b7fad"), "Uniswap: Universal Router"),
    (address!("0x000000000022d473030f116ddee9f6b43ac78ba3"), "Permit2"),
    (address!("0x00000000000000adc04c56bf30ac9d3c0aaf14dc"), "Seaport 1.5"),
    (address!("0x1111111254eeb25477b68fb85ed929f73a960582"), "1inch V5: Aggregation Router"),
    (address!("0xdef1c0ded9bec7f1a1670819833240f027b25eff"), "0x: Exchange Proxy"),
    (address!("0xca11bde05977b3631167028862be2a173976ca11"), "Multicall3"),
];

/// Label for the zero address, used for mints/burns.
pub const NULL_ADDRESS_LABEL: &str = "Null: mint/burn";

/// Built-in label for `addr` on `chain_id`, if known.
pub fn label(chain_id: u64, addr: Address) -> Option<&'static str> {
    if addr == Address::ZERO {
        return Some(NULL_ADDRESS_LABEL);
    }
    if let Some(name) = precompile_name(addr) {
        return Some(name);
    }
    if chain_id == 1 {
        return MAINNET_LABELS.iter().find(|(a, _)| *a == addr).map(|(_, l)| *l);
    }
    None
}

/// Names for the standard precompiled contracts (0x01..=0x11).
pub fn precompile_name(addr: Address) -> Option<&'static str> {
    let bytes = addr.as_slice();
    if bytes[..19].iter().any(|b| *b != 0) {
        return None;
    }
    Some(match bytes[19] {
        0x01 => "Precompile: ecrecover",
        0x02 => "Precompile: sha256",
        0x03 => "Precompile: ripemd160",
        0x04 => "Precompile: identity",
        0x05 => "Precompile: modexp",
        0x06 => "Precompile: ecadd",
        0x07 => "Precompile: ecmul",
        0x08 => "Precompile: ecpairing",
        0x09 => "Precompile: blake2f",
        0x0a => "Precompile: kzg-point-evaluation",
        0x0b => "Precompile: bls12-g1add",
        0x0c => "Precompile: bls12-g1msm",
        0x0d => "Precompile: bls12-g2add",
        0x0e => "Precompile: bls12-g2msm",
        0x0f => "Precompile: bls12-pairing",
        0x10 => "Precompile: bls12-map-fp-to-g1",
        0x11 => "Precompile: bls12-map-fp2-to-g2",
        _ => return None,
    })
}

/// Whether `addr` is a standard precompile.
pub fn is_precompile(addr: Address) -> bool {
    precompile_name(addr).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_resolve() {
        assert_eq!(native_symbol(1), "ETH");
        assert_eq!(native_symbol(137), "POL");
        let weth = wrapped_native(1).unwrap();
        assert_eq!(label(1, weth), Some("WETH"));
        assert_eq!(label(1, Address::ZERO), Some(NULL_ADDRESS_LABEL));
        assert_eq!(label(31337, Address::ZERO), Some(NULL_ADDRESS_LABEL));
        assert!(is_precompile(address!("0x0000000000000000000000000000000000000001")));
        assert!(!is_precompile(weth));
    }
}
