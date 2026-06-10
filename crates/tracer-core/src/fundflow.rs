//! Fund-flow view: an ordered transfer graph, the feature Phalcon and
//! Tenderly render as the "Fund Flow" / "Asset Flow" diagram.
//!
//! Edges are one-per-transfer (not pre-aggregated) so UIs can choose their own
//! aggregation; `order` preserves execution order for numbered arrows.

use crate::{amount::Amount, transfer::Asset, transfer::TransferOrigin};
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    /// Known externally-owned account (e.g. the tx sender).
    Eoa,
    /// Known contract (appeared as a call target or token).
    Contract,
    /// A token contract participating as a party (wrap/unwrap, mint/burn).
    Token,
    /// Unknown — never called, only received/sent value.
    Account,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowNode {
    /// The address (also the node id for edge references).
    pub id: Address,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub label: Option<String>,
    pub kind: NodeKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowEdge {
    pub id: u32,
    /// Execution order of the underlying transfer.
    pub order: u32,
    pub from: Address,
    pub to: Address,
    pub asset: Asset,
    pub amount: Amount,
    pub origin: TransferOrigin,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundFlow {
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
}
