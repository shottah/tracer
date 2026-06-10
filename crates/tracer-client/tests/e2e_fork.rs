//! Anvil-fork backend e2e: build a multi-transaction block on one anvil
//! node, then trace the second transaction through the fork-and-replay
//! backend (forking that node) and cross-check against the direct RPC
//! result. Exercises block-env pinning, preceding-tx replay with nonce
//! chaining, FIFO ordering, and the fidelity check.

mod common;

use alloy::providers::Provider;
use common::*;
use tracer_client::{BackendChoice, Tracer, TracerConfig};
use tracer_core::BackendKind;

#[tokio::test]
async fn fork_replay_matches_direct_rpc() {
    // Source node: manual mining so we can build a 2-tx block.
    let Some(anvil) = spawn_anvil(&["--no-mining", "--order", "fifo"]) else { return };
    let provider = provider_for(&anvil);
    let accounts = provider.get_accounts().await.expect("accounts");
    let (sender_addr, eoa1, eoa2) = (accounts[0], accounts[1], accounts[2]);

    // Block 1: the three deployments.
    let mut sender = TxSender::new(provider.clone(), sender_addr).await;
    let token_hash = sender.send(None, 0, asm::initcode(&token_runtime()), 1_000_000).await;
    let rev_hash = sender.send(None, 0, asm::initcode(&reverter_runtime()), 1_000_000).await;
    mine(&provider).await;
    let token = receipt_of(&provider, token_hash).await.contract_address.expect("token");
    let reverter = receipt_of(&provider, rev_hash).await.contract_address.expect("reverter");

    // Block 2: deploy MAIN.
    let main_hash = sender
        .send(None, 0, asm::initcode(&main_runtime(eoa1, token, eoa2, reverter)), 1_000_000)
        .await;
    increase_time(&provider, 12).await;
    mine(&provider).await;
    let main = receipt_of(&provider, main_hash).await.contract_address.expect("main");

    // Block 3: a preceding transfer AND the target, in one block.
    let preceding = sender.send(Some(eoa1), 7, vec![], 21_000).await;
    let target = sender.send(Some(main), 1000, vec![], 500_000).await;
    increase_time(&provider, 12).await;
    mine(&provider).await;
    let preceding_receipt = receipt_of(&provider, preceding).await;
    let target_receipt = receipt_of(&provider, target).await;
    assert!(preceding_receipt.status() && target_receipt.status());
    assert_eq!(target_receipt.transaction_index, Some(1), "target must be second in block");

    // Direct RPC report (anvil has a debug API).
    let direct = Tracer::connect(TracerConfig {
        rpc_url: anvil.endpoint(),
        backend: BackendChoice::Rpc,
        enrich: false,
        ..Default::default()
    })
    .expect("connect direct")
    .report(target)
    .await
    .expect("direct report");

    // Fork-and-replay report: forks the node above at block 2, replays the
    // preceding transfer, re-executes the target, traces locally.
    let forked = Tracer::connect(TracerConfig {
        rpc_url: anvil.endpoint(),
        backend: BackendChoice::AnvilFork,
        enrich: false,
        ..Default::default()
    })
    .expect("connect fork")
    .report(target)
    .await
    .expect("fork report");

    // Backend bookkeeping.
    assert_eq!(forked.backend.kind, BackendKind::AnvilFork);
    let fork = forked.backend.fork.as_ref().expect("fork info");
    assert_eq!(fork.replayed, 1, "one preceding tx must replay (warnings: {:?})", forked.warnings);
    assert_eq!(fork.skipped, 0);
    let fidelity = fork.fidelity.expect("fidelity check");
    assert!(
        fidelity.ok(),
        "fork replay must match the original receipt: {fidelity:?} (warnings: {:?})",
        forked.warnings
    );

    // The report still describes the ORIGINAL transaction.
    assert_eq!(forked.tx.hash, target);
    assert_eq!(forked.tx.block_number, direct.tx.block_number);
    assert_eq!(forked.tx.gas_used, direct.tx.gas_used);

    // Same call tree shape...
    let (d_root, f_root) = (direct.trace.as_ref().unwrap(), forked.trace.as_ref().unwrap());
    assert_eq!(d_root.children.len(), f_root.children.len());
    for (d, f) in d_root.children.iter().zip(&f_root.children) {
        assert_eq!(d.kind, f.kind);
        assert_eq!(d.to, f.to);
        assert_eq!(d.value, f.value);
        assert_eq!(d.error.is_some(), f.error.is_some());
    }

    // ...identical transfers...
    assert_eq!(
        serde_json::to_value(&direct.transfers).unwrap(),
        serde_json::to_value(&forked.transfers).unwrap(),
        "transfers must match between direct RPC and fork replay"
    );

    // ...and identical balance deltas.
    let deltas = |r: &tracer_core::TraceReport| -> Vec<(String, String)> {
        r.balance_changes
            .as_ref()
            .unwrap()
            .changes
            .iter()
            .filter_map(|c| {
                c.native.as_ref().map(|n| (format!("{:?}", c.address), n.delta.dec.clone()))
            })
            .collect()
    };
    assert_eq!(deltas(&direct), deltas(&forked), "native deltas must match");
}
