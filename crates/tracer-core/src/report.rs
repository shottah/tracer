//! The top-level report envelope: everything a UI needs to render a
//! transaction, in one versioned JSON document.

use crate::{balance::BalanceChanges, fundflow::FundFlow, trace::Frame, transfer::AssetTransfer};
use alloy_primitives::{Address, B256, Bytes, U256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bumped on breaking changes to the JSON shape.
pub const SCHEMA_VERSION: &str = "0.1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxSummary {
    pub hash: B256,
    pub from: Address,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub to: Option<Address>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub contract_created: Option<Address>,
    pub value: U256,
    pub input: Bytes,
    pub nonce: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub effective_gas_price: u128,
    pub status: bool,
    pub tx_type: u8,
    pub block_number: u64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub block_timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub transaction_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub base_fee_per_gas: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackendKind {
    Rpc,
    AnvilFork,
}

/// Replay-fidelity comparison between the original receipt and the receipt of
/// the re-executed transaction on the fork.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FidelityCheck {
    pub status_match: bool,
    pub gas_used_match: bool,
    pub log_count_match: bool,
}

impl FidelityCheck {
    pub fn ok(&self) -> bool {
        self.status_match && self.gas_used_match && self.log_count_match
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkInfo {
    /// Block the fork was taken at (target block - 1).
    pub fork_block: u64,
    /// Preceding same-block transactions replayed before the target.
    pub replayed: u32,
    /// Preceding transactions that could not be replayed.
    pub skipped: u32,
    /// Hash of the re-executed transaction on the fork.
    pub replayed_tx_hash: B256,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fidelity: Option<FidelityCheck>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendInfo {
    pub kind: BackendKind,
    /// Scheme+host of the endpoint only — never the full URL, which may
    /// embed an API key.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub endpoint_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fork: Option<ForkInfo>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenStandard {
    Erc20,
    Erc721,
    Erc1155,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub address: Address,
    pub standard: TokenStandard,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub decimals: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceReport {
    pub schema_version: String,
    pub chain_id: u64,
    /// Display symbol for the native currency ("ETH", "POL", ...).
    pub native_symbol: String,
    pub tx: TxSummary,
    pub backend: BackendInfo,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub trace: Option<Frame>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub transfers: Vec<AssetTransfer>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub balance_changes: Option<BalanceChanges>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fund_flow: Option<FundFlow>,
    /// Metadata for every token seen in `transfers`, keyed by address.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub tokens: BTreeMap<Address, TokenInfo>,
    /// Labels for well-known addresses involved in this transaction.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub address_labels: BTreeMap<Address, String>,
    /// Non-fatal degradations encountered while producing the report.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
}
