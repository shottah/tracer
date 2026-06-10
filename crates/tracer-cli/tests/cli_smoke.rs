//! Smoke test of the actual `tracer` binary against a local anvil node.
//! Skips when anvil is not installed (CI installs Foundry).

use alloy::{
    network::TransactionBuilder,
    node_bindings::Anvil,
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
};
use alloy_primitives::U256;
use std::process::Command;

#[tokio::test]
async fn binary_renders_all_formats() {
    let anvil = match Anvil::new().try_spawn() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("SKIPPED: anvil unavailable ({e})");
            return;
        }
    };
    let provider = ProviderBuilder::new().connect_http(anvil.endpoint_url()).erased();
    let accounts = provider.get_accounts().await.expect("accounts");

    // Simple value transfer (anvil signs for its unlocked account).
    let req = TransactionRequest::default()
        .with_from(accounts[0])
        .with_to(accounts[1])
        .with_value(U256::from(1_500_000_000_000_000_000u128)); // 1.5 ETH
    let pending = provider.send_transaction(req).await.expect("send");
    let hash = *pending.tx_hash();
    // Wait for the receipt so the CLI sees a mined transaction.
    for _ in 0..100 {
        if provider.get_transaction_receipt(hash).await.expect("receipt").is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let bin = env!("CARGO_BIN_EXE_tracer");
    let run = |args: &[&str]| -> (bool, String) {
        let out = Command::new(bin)
            .args(args)
            .args(["--rpc-url", &anvil.endpoint(), "--quiet"])
            .output()
            .expect("binary runs");
        (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned())
    };
    let tx = format!("{hash:?}");

    // Full JSON report.
    let (ok, stdout) = run(&["report", &tx]);
    assert!(ok, "report failed: {stdout}");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(report["schemaVersion"], "0.1");
    assert_eq!(report["transfers"].as_array().map(Vec::len), Some(1));
    assert_eq!(report["transfers"][0]["amount"]["formatted"], "1.5");
    assert!(report["balanceChanges"]["changes"].as_array().is_some());
    assert!(report["fundFlow"]["edges"][0]["order"].is_number());

    // Human tree.
    let (ok, stdout) = run(&["trace", &tx]);
    assert!(ok);
    assert!(stdout.contains("CALL"), "tree output: {stdout}");
    assert!(stdout.contains("transfers (1)"), "tree output: {stdout}");

    // Balance table.
    let (ok, stdout) = run(&["balance-changes", &tx]);
    assert!(ok);
    assert!(stdout.contains("account") && stdout.contains("delta"), "table: {stdout}");

    // Mermaid + DOT.
    let (ok, stdout) = run(&["fund-flow", &tx]);
    assert!(ok);
    assert!(stdout.contains("flowchart LR"), "mermaid: {stdout}");
    let (ok, stdout) = run(&["fund-flow", &tx, "--format", "dot"]);
    assert!(ok);
    assert!(stdout.contains("digraph fundflow"), "dot: {stdout}");

    // Unknown tx exits with code 2.
    let missing = "0x00000000000000000000000000000000000000000000000000000000000000aa";
    let out = Command::new(bin)
        .args(["report", missing, "--rpc-url", &anvil.endpoint(), "--quiet"])
        .output()
        .expect("binary runs");
    assert_eq!(out.status.code(), Some(2), "missing tx should exit 2");
}
