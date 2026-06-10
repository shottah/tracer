//! Full-pipeline e2e against a live local anvil (direct RPC backend):
//! deploys hand-assembled contracts, runs a transaction exercising native
//! sends, a token transfer, an inner revert, and storage writes, then checks
//! the report — including balance deltas against `eth_getBalance` ground
//! truth.

mod common;

use alloy::providers::Provider;
use common::*;
use tracer_client::{BackendChoice, Tracer, TracerConfig};
use tracer_core::{Asset, CallKind, TransferOrigin};

#[tokio::test]
async fn report_matches_chain_ground_truth() {
    // --steps-tracing enables geth-style struct logs (needed for --deep).
    let Some(anvil) = spawn_anvil(&["--steps-tracing"]) else { return };
    let provider = provider_for(&anvil);
    let accounts = provider.get_accounts().await.expect("accounts");
    let (sender_addr, eoa1, eoa2) = (accounts[0], accounts[1], accounts[2]);

    // Deploy the three contracts (anvil automines each).
    let mut sender = TxSender::new(provider.clone(), sender_addr).await;
    let token_hash = sender.send(None, 0, asm::initcode(&token_runtime()), 1_000_000).await;
    let token = receipt_of(&provider, token_hash).await.contract_address.expect("token addr");
    let rev_hash = sender.send(None, 0, asm::initcode(&reverter_runtime()), 1_000_000).await;
    let reverter = receipt_of(&provider, rev_hash).await.contract_address.expect("reverter addr");
    let main_hash = sender
        .send(None, 0, asm::initcode(&main_runtime(eoa1, token, eoa2, reverter)), 1_000_000)
        .await;
    let main = receipt_of(&provider, main_hash).await.contract_address.expect("main addr");

    // Ground truth balances before the target transaction.
    let pre_sender = balance(&provider, sender_addr).await;
    let pre_main = balance(&provider, main).await;
    let pre_eoa1 = balance(&provider, eoa1).await;

    let target = sender.send(Some(main), 1000, vec![], 500_000).await;
    let receipt = receipt_of(&provider, target).await;
    assert!(receipt.status(), "target tx must succeed");

    let post_sender = balance(&provider, sender_addr).await;
    let post_main = balance(&provider, main).await;
    let post_eoa1 = balance(&provider, eoa1).await;

    // Run the full pipeline (deep mode, no metadata enrichment — the toy
    // token answers every call with a Transfer event).
    let tracer = Tracer::connect(TracerConfig {
        rpc_url: anvil.endpoint(),
        backend: BackendChoice::Rpc,
        deep: true,
        enrich: false,
        ..Default::default()
    })
    .expect("connect");
    let report = tracer.report(target).await.expect("report");

    // --- transaction summary ---
    assert_eq!(report.schema_version, "0.1");
    assert!(report.tx.status);
    assert_eq!(report.tx.to, Some(main));
    assert_eq!(report.tx.gas_used, receipt.gas_used);

    // --- call tree ---
    let root = report.trace.as_ref().expect("trace");
    assert_eq!(root.kind, CallKind::Call);
    assert_eq!(root.to, Some(main));
    assert_eq!(root.children.len(), 3, "tree: {root:#?}");

    let send_call = &root.children[0];
    assert_eq!(send_call.to, Some(eoa1));
    assert_eq!(send_call.value, alloy_primitives::U256::from(1u8));
    assert!(send_call.ok());

    let token_call = &root.children[1];
    assert_eq!(token_call.to, Some(token));
    assert_eq!(token_call.decoded.as_ref().unwrap().name.as_deref(), Some("transfer"));
    assert_eq!(token_call.logs.len(), 1);
    let log = &token_call.logs[0];
    assert_eq!(log.decoded.as_ref().unwrap().name, "Transfer");
    assert!(log.log_index.is_some(), "receipt log index should be assigned");

    let revert_call = &root.children[2];
    assert_eq!(revert_call.to, Some(reverter));
    assert!(revert_call.error.is_some(), "inner revert must be marked");

    // Deep mode: the SSTORE(7, 42) lands on the root frame.
    assert!(
        root.storage_writes.iter().any(|w| {
            w.slot == alloy_primitives::B256::from(alloy_primitives::U256::from(7u8))
                && w.value == alloy_primitives::B256::from(alloy_primitives::U256::from(42u8))
        }),
        "deep storage write missing: {:?} (warnings: {:?})",
        root.storage_writes,
        report.warnings,
    );

    // --- transfers (execution order) ---
    assert_eq!(report.transfers.len(), 3, "{:#?}", report.transfers);
    assert_eq!(report.transfers[0].origin, TransferOrigin::TxValue);
    assert_eq!(report.transfers[0].amount.dec, "1000");
    assert_eq!((report.transfers[1].from, report.transfers[1].to), (main, eoa1));
    assert_eq!(report.transfers[2].asset, Asset::Erc20 { token });
    assert_eq!((report.transfers[2].from, report.transfers[2].to), (main, eoa2));
    assert_eq!(report.transfers[2].amount.dec, "5");

    // --- balance changes vs chain ground truth ---
    let bc = report.balance_changes.as_ref().expect("balance changes");
    let row = |addr| bc.changes.iter().find(|c| c.address == addr);
    assert_eq!(bc.changes[0].address, sender_addr, "sender row first");

    let sender_delta = &row(sender_addr).unwrap().native.as_ref().unwrap().delta;
    assert_eq!(sender_delta.dec, delta_dec(pre_sender, post_sender), "sender ETH delta");
    let main_delta = &row(main).unwrap().native.as_ref().unwrap().delta;
    assert_eq!(main_delta.dec, delta_dec(pre_main, post_main), "main ETH delta (=+999)");
    assert_eq!(main_delta.dec, "999");
    let eoa1_delta = &row(eoa1).unwrap().native.as_ref().unwrap().delta;
    assert_eq!(eoa1_delta.dec, delta_dec(pre_eoa1, post_eoa1));
    assert_eq!(eoa1_delta.dec, "1");

    let main_tokens = &row(main).unwrap().tokens;
    assert_eq!(main_tokens.len(), 1);
    assert_eq!(main_tokens[0].delta.dec, "-5");
    let eoa2_tokens = &row(eoa2).unwrap().tokens;
    assert_eq!(eoa2_tokens[0].delta.dec, "5");

    // --- fund flow ---
    let flow = report.fund_flow.as_ref().expect("fund flow");
    assert_eq!(flow.edges.len(), 3);
    assert_eq!(flow.edges.iter().map(|e| e.order).collect::<Vec<_>>(), vec![0, 1, 2]);
    let node_kind = |addr| flow.nodes.iter().find(|n| n.id == addr).map(|n| n.kind);
    assert_eq!(node_kind(sender_addr), Some(tracer_core::NodeKind::Eoa));
    assert_eq!(node_kind(main), Some(tracer_core::NodeKind::Contract));

    // --- schema round-trip ---
    let json = serde_json::to_string_pretty(&report).expect("serialize");
    let parsed: tracer_core::TraceReport = serde_json::from_str(&json).expect("round-trip");
    assert_eq!(parsed.transfers.len(), 3);
    assert!(json.contains("\"schemaVersion\""), "camelCase JSON expected");
}
