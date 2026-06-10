//! Anvil fork backend: re-execute a historical transaction on a local fork
//! so any plain state RPC can serve full traces — no remote `debug` API
//! needed.
//!
//! Strategy: fork at `block - 1`, pin the next block's environment
//! (timestamp, base fee, coinbase, gas limit) to the original block, replay
//! the block's preceding transactions FIFO via impersonated
//! `eth_sendTransaction`, queue the target, mine once, then trace against
//! anvil. A fidelity check against the original receipt is recorded so
//! consumers can detect divergence.

use crate::{ClientError, artifacts::TxMeta};
use alloy::{
    consensus::Transaction as _,
    eips::Typed2718 as _,
    network::TransactionBuilder,
    node_bindings::{Anvil, AnvilInstance},
    providers::{DynProvider, Provider, ProviderBuilder},
    rpc::types::{Transaction as RpcTransaction, TransactionRequest},
};
use alloy_primitives::{B256, U256};
use tracer_core::{FidelityCheck, ForkInfo};

pub struct ForkSession {
    /// Keeps the anvil child process alive while traces are fetched.
    _instance: AnvilInstance,
    pub provider: DynProvider,
    /// Hash of the re-executed transaction on the fork (differs from the
    /// original hash: anvil re-signs impersonated transactions).
    pub traced_hash: B256,
    pub fork: ForkInfo,
    pub warnings: Vec<String>,
}

pub async fn fork_and_replay(
    rpc_url: &str,
    remote: &DynProvider,
    meta: &TxMeta,
    replay: bool,
) -> Result<ForkSession, ClientError> {
    let mut warnings = Vec::new();
    let target_hash = meta.summary.hash;
    let block_number = meta.summary.block_number;
    let fork_block = block_number
        .checked_sub(1)
        .ok_or_else(|| ClientError::Anvil("cannot fork before genesis".into()))?;
    let target_index = meta.summary.transaction_index.unwrap_or(0);

    // Full block (with transactions) is only needed to replay predecessors.
    let block = remote
        .get_block_by_number(block_number.into())
        .full()
        .await
        .map_err(ClientError::rpc)?
        .ok_or(ClientError::BlockNotFound(block_number))?;

    let anvil = Anvil::new()
        .fork(rpc_url)
        .fork_block_number(fork_block)
        .args([
            "--no-mining",
            "--order",
            "fifo",
            "--auto-impersonate",
            // Required for geth-style struct logs (--deep) on anvil.
            "--steps-tracing",
            "--gas-limit",
            &block.header.gas_limit.to_string(),
        ])
        .try_spawn()
        .map_err(|e| ClientError::Anvil(e.to_string()))?;
    let provider = ProviderBuilder::new().connect_http(anvil.endpoint_url()).erased();

    // Pin the next block's environment to the original block.
    pin_env(&provider, meta, &block, &mut warnings).await;

    let mut replayed = 0u32;
    let mut skipped = 0u32;
    if target_index > 0 {
        if replay {
            let txs = block
                .transactions
                .as_transactions()
                .ok_or_else(|| ClientError::Anvil("full block lacked transaction bodies".into()))?;
            let mut preceding: Vec<&RpcTransaction> = txs
                .iter()
                .filter(|t| t.transaction_index.unwrap_or(u64::MAX) < target_index)
                .collect();
            preceding.sort_by_key(|t| t.transaction_index.unwrap_or(u64::MAX));
            for t in preceding {
                let idx = t.transaction_index.unwrap_or_default();
                if t.ty() == 3 {
                    warnings.push(format!(
                        "tx #{idx} in the block is a blob transaction; replayed without its \
                         sidecar (BLOBHASH reads zero)"
                    ));
                }
                match send_replica(&provider, t).await {
                    Ok(_) => replayed += 1,
                    Err(e) => {
                        skipped += 1;
                        warnings.push(format!(
                            "could not replay preceding tx #{idx}: {e}; fork state may diverge"
                        ));
                    }
                }
            }
        } else {
            warnings.push(format!(
                "replay disabled: {target_index} preceding transaction(s) in the block were \
                 skipped; fork state may diverge"
            ));
        }
    }

    // Queue and mine the target.
    let target_tx = remote
        .get_transaction_by_hash(target_hash)
        .await
        .map_err(ClientError::rpc)?
        .ok_or(ClientError::TxNotFound(target_hash))?;
    if target_tx.ty() == 3 {
        warnings.push(
            "target is a blob transaction; replayed without its sidecar (BLOBHASH reads zero)"
                .into(),
        );
    }
    let traced_hash = send_replica(&provider, &target_tx)
        .await
        .map_err(|e| ClientError::Replay(e.to_string()))?;
    provider
        .raw_request::<_, serde_json::Value>("evm_mine".into(), Vec::<serde_json::Value>::new())
        .await
        .map_err(|e| ClientError::Replay(format!("evm_mine failed: {e}")))?;

    let local_receipt = provider
        .get_transaction_receipt(traced_hash)
        .await
        .map_err(ClientError::rpc)?
        .ok_or_else(|| ClientError::Replay("transaction did not mine on the fork".into()))?;

    let fidelity = FidelityCheck {
        status_match: local_receipt.status() == meta.summary.status,
        gas_used_match: local_receipt.gas_used == meta.summary.gas_used,
        log_count_match: local_receipt.logs().len() == meta.receipt_logs.len(),
    };
    if !fidelity.ok() {
        warnings.push(format!(
            "fork replay diverged from the original receipt \
             (status match: {}, gasUsed match: {}, log count match: {}); \
             treat trace details with care",
            fidelity.status_match, fidelity.gas_used_match, fidelity.log_count_match
        ));
    }

    Ok(ForkSession {
        _instance: anvil,
        provider,
        traced_hash,
        fork: ForkInfo {
            fork_block,
            replayed,
            skipped,
            replayed_tx_hash: traced_hash,
            fidelity: Some(fidelity),
        },
        warnings,
    })
}

async fn pin_env(
    provider: &DynProvider,
    meta: &TxMeta,
    block: &alloy::rpc::types::Block,
    warnings: &mut Vec<String>,
) {
    let mut admin = async |method: &'static str, params: serde_json::Value| {
        if let Err(e) = provider.raw_request::<_, serde_json::Value>(method.into(), params).await {
            warnings.push(format!("{method} failed on anvil: {e}; block env fidelity reduced"));
        }
    };
    admin("evm_setNextBlockTimestamp", serde_json::json!([block.header.timestamp])).await;
    if let Some(base_fee) = meta.summary.base_fee_per_gas {
        admin("anvil_setNextBlockBaseFeePerGas", serde_json::json!([U256::from(base_fee)])).await;
    }
    admin("anvil_setCoinbase", serde_json::json!([block.header.beneficiary])).await;
}

/// Rebuild a mined transaction as an impersonated `eth_sendTransaction`.
/// All fields are set explicitly so provider fillers stay inert.
async fn send_replica(provider: &DynProvider, tx: &RpcTransaction) -> Result<B256, ClientError> {
    let mut req = TransactionRequest::default()
        .with_from(tx.inner.signer())
        .with_kind(tx.kind())
        .with_value(tx.value())
        .with_nonce(tx.nonce())
        .with_gas_limit(tx.gas_limit())
        .with_input(tx.input().clone());
    if let Some(chain_id) = tx.chain_id() {
        req = req.with_chain_id(chain_id);
    }
    match tx.ty() {
        0 | 1 => {
            if let Some(gas_price) = tx.gas_price() {
                req = req.with_gas_price(gas_price);
            }
        }
        _ => {
            req = req.with_max_fee_per_gas(tx.max_fee_per_gas());
            if let Some(tip) = tx.max_priority_fee_per_gas() {
                req = req.with_max_priority_fee_per_gas(tip);
            }
        }
    }
    if let Some(access_list) = tx.access_list() {
        req.access_list = Some(access_list.clone());
    }
    if let Some(auths) = tx.authorization_list() {
        req.authorization_list = Some(auths.to_vec());
    }
    let pending = provider.send_transaction(req).await.map_err(ClientError::rpc)?;
    Ok(*pending.tx_hash())
}
