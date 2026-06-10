//! Build the fund-flow graph from ordered transfers.

use alloy_primitives::Address;
use std::collections::HashSet;
use tracer_core::{AssetTransfer, FlowEdge, FlowNode, FundFlow, NodeKind};

#[derive(Clone, Debug, Default)]
pub struct FundFlowContext {
    pub tx_from: Address,
    /// Addresses known to have code (from the trace).
    pub contracts: HashSet<Address>,
}

/// One node per address touched by a transfer (first-appearance order),
/// one edge per transfer (execution order preserved).
pub fn build_fund_flow(transfers: &[AssetTransfer], ctx: &FundFlowContext) -> FundFlow {
    let token_addrs: HashSet<Address> = transfers.iter().filter_map(|t| t.asset.token()).collect();

    let mut seen: HashSet<Address> = HashSet::new();
    let mut nodes: Vec<FlowNode> = Vec::new();
    let add_node = |addr: Address, nodes: &mut Vec<FlowNode>, seen: &mut HashSet<Address>| {
        if !seen.insert(addr) {
            return;
        }
        let kind = if token_addrs.contains(&addr) {
            NodeKind::Token
        } else if ctx.contracts.contains(&addr) {
            NodeKind::Contract
        } else if addr == ctx.tx_from {
            NodeKind::Eoa
        } else {
            NodeKind::Account
        };
        nodes.push(FlowNode { id: addr, label: None, kind });
    };

    let mut edges = Vec::with_capacity(transfers.len());
    for (i, t) in transfers.iter().enumerate() {
        add_node(t.from, &mut nodes, &mut seen);
        add_node(t.to, &mut nodes, &mut seen);
        edges.push(FlowEdge {
            id: i as u32,
            order: t.order,
            from: t.from,
            to: t.to,
            asset: t.asset.clone(),
            amount: t.amount.clone(),
            origin: t.origin,
        });
    }

    FundFlow { nodes, edges }
}
