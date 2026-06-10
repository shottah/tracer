//! Aggregate transfers (plus gas accounting or prestate diffs) into the
//! balance-changes view.

use alloy_primitives::{Address, U256};
use std::collections::BTreeMap;
use tracer_core::{
    AccountBalanceChange, Amount, Asset, AssetTransfer, BalanceChanges, NativeChange, NativeSource,
    SignedAmount, TokenChange,
};

/// Exact native pre/post balances per account (from `prestateTracer` diff
/// mode). Accounts absent from the diff did not change.
pub type NativeDiff = BTreeMap<Address, (Option<U256>, Option<U256>)>;

#[derive(Clone, Debug)]
pub struct BalanceInput<'a> {
    pub transfers: &'a [AssetTransfer],
    pub tx_from: Address,
    pub coinbase: Option<Address>,
    pub gas_used: u64,
    pub effective_gas_price: u128,
    pub base_fee_per_gas: Option<u64>,
    /// Prefer exact balances when available.
    pub prestate_native: Option<&'a NativeDiff>,
    /// Whether to fold gas fees into derived native deltas.
    pub include_gas: bool,
}

/// Compute per-account balance changes.
///
/// Native deltas come from the prestate diff when provided (exact, gas
/// included by construction); otherwise they are derived from native
/// transfers plus gas fees for the sender and the priority fee for the
/// coinbase. Token deltas always aggregate the transfer list.
pub fn compute_balance_changes(input: &BalanceInput<'_>) -> BalanceChanges {
    // Token deltas per (account, asset).
    let mut tokens: BTreeMap<Address, BTreeMap<String, TokenChange>> = BTreeMap::new();
    let mut bump = |addr: Address, asset: &Asset, amount: U256, positive: bool| {
        if addr == Address::ZERO {
            return; // mint/burn counterparty: not a balance row
        }
        let entry = tokens.entry(addr).or_default().entry(asset.key()).or_insert_with(|| {
            TokenChange { asset: asset.clone(), delta: SignedAmount::default(), transfer_count: 0 }
        });
        if positive {
            entry.delta.add(amount);
        } else {
            entry.delta.sub(amount);
        }
        entry.transfer_count += 1;
    };
    for t in input.transfers {
        if matches!(t.asset, Asset::Native) {
            continue;
        }
        bump(t.from, &t.asset, t.amount.raw, false);
        bump(t.to, &t.asset, t.amount.raw, true);
    }

    // Native deltas.
    let mut native: BTreeMap<Address, NativeChange> = BTreeMap::new();
    let gas_fee = U256::from(input.gas_used as u128 * input.effective_gas_price);
    let native_source;
    let gas_included;

    if let Some(diff) = input.prestate_native {
        native_source = NativeSource::Prestate;
        gas_included = true;
        for (addr, (pre, post)) in diff {
            let pre_v = pre.unwrap_or_default();
            let post_v = post.unwrap_or_default();
            let mut delta = SignedAmount::default();
            if post_v >= pre_v {
                delta.add(post_v - pre_v);
            } else {
                delta.sub(pre_v - post_v);
            }
            native.insert(
                *addr,
                NativeChange {
                    pre: *pre,
                    post: *post,
                    delta,
                    gas_fee: (*addr == input.tx_from).then(|| Amount::new(gas_fee)),
                },
            );
        }
    } else {
        native_source = NativeSource::Derived;
        gas_included = input.include_gas;
        let mut deltas: BTreeMap<Address, SignedAmount> = BTreeMap::new();
        for t in input.transfers {
            if !matches!(t.asset, Asset::Native) {
                continue;
            }
            deltas.entry(t.from).or_default().sub(t.amount.raw);
            deltas.entry(t.to).or_default().add(t.amount.raw);
        }
        if input.include_gas {
            deltas.entry(input.tx_from).or_default().sub(gas_fee);
            if let Some(coinbase) = input.coinbase {
                let tip_per_gas = input
                    .effective_gas_price
                    .saturating_sub(input.base_fee_per_gas.unwrap_or_default() as u128);
                let tip = U256::from(input.gas_used as u128 * tip_per_gas);
                deltas.entry(coinbase).or_default().add(tip);
            }
        }
        for (addr, delta) in deltas {
            native.insert(
                addr,
                NativeChange {
                    pre: None,
                    post: None,
                    delta,
                    gas_fee: (addr == input.tx_from && input.include_gas)
                        .then(|| Amount::new(gas_fee)),
                },
            );
        }
    }

    // Merge into rows: sender first, then by address.
    let mut addresses: Vec<Address> = native
        .keys()
        .chain(tokens.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    addresses.sort_by_key(|a| (*a != input.tx_from, *a));

    let mut changes = Vec::new();
    for addr in addresses {
        let native_change = native
            .remove(&addr)
            .filter(|n| !n.delta.is_zero() || n.pre.is_some() || n.gas_fee.is_some());
        let token_changes: Vec<TokenChange> = tokens
            .remove(&addr)
            .map(|m| {
                m.into_values().filter(|t| !t.delta.is_zero() || t.transfer_count > 0).collect()
            })
            .unwrap_or_default();
        if native_change.is_none() && token_changes.is_empty() {
            continue;
        }
        changes.push(AccountBalanceChange {
            address: addr,
            label: None,
            native: native_change,
            tokens: token_changes,
        });
    }

    BalanceChanges { native_source, gas_included, changes }
}
