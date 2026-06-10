//! Balance-changes view: per-account native and token deltas, the feature
//! Phalcon and Tenderly render as the "Balance Changes" panel.

use crate::{amount::Amount, amount::SignedAmount, transfer::Asset};
use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

/// How native deltas were obtained.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeSource {
    /// Exact pre/post balances from `prestateTracer` in diff mode.
    Prestate,
    /// Derived from observed value transfers plus gas accounting.
    Derived,
}

/// Native currency delta for one account.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeChange {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub pre: Option<U256>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub post: Option<U256>,
    pub delta: SignedAmount,
    /// Portion of the delta that is the gas fee (set on the sender row).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub gas_fee: Option<Amount>,
}

/// Token delta for one `(account, asset)` pair.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenChange {
    pub asset: Asset,
    pub delta: SignedAmount,
    /// Number of transfers contributing to this delta.
    pub transfer_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBalanceChange {
    pub address: Address,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub native: Option<NativeChange>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tokens: Vec<TokenChange>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceChanges {
    pub native_source: NativeSource,
    /// Whether gas fees are reflected in native deltas.
    pub gas_included: bool,
    pub changes: Vec<AccountBalanceChange>,
}
