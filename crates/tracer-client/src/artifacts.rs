//! Fetch per-transaction artifacts: metadata, receipt logs, and the three
//! debug tracer outputs (callTracer, prestateTracer diff, struct logs).

use crate::{ClientError, rpc::is_unsupported_error};
use alloy::{
    consensus::Transaction as _,
    eips::Typed2718 as _,
    providers::{DynProvider, Provider, ext::DebugApi},
    rpc::types::trace::geth::{
        CallConfig, DefaultFrame, DiffMode, GethDebugBuiltInTracerType, GethDebugTracerConfig,
        GethDebugTracerType, GethDebugTracingOptions, GethDefaultTracingOptions, GethTrace,
        PreStateConfig, PreStateFrame,
    },
};
use alloy_primitives::{Address, B256};
use tracer_core::{ReceiptLog, TxSummary};

/// Chain- and receipt-level facts about the transaction being traced.
#[derive(Clone, Debug)]
pub struct TxMeta {
    pub chain_id: u64,
    pub summary: TxSummary,
    pub coinbase: Option<Address>,
    pub receipt_logs: Vec<ReceiptLog>,
}

pub async fn fetch_meta(provider: &DynProvider, hash: B256) -> Result<TxMeta, ClientError> {
    let tx = provider
        .get_transaction_by_hash(hash)
        .await
        .map_err(ClientError::rpc)?
        .ok_or(ClientError::TxNotFound(hash))?;
    let receipt = provider
        .get_transaction_receipt(hash)
        .await
        .map_err(ClientError::rpc)?
        .ok_or(ClientError::NotMined(hash))?;
    let block_number = receipt.block_number.ok_or(ClientError::NotMined(hash))?;
    let block = provider
        .get_block_by_number(block_number.into())
        .await
        .map_err(ClientError::rpc)?
        .ok_or(ClientError::BlockNotFound(block_number))?;
    let chain_id = provider.get_chain_id().await.map_err(ClientError::rpc)?;

    let summary = TxSummary {
        hash,
        from: tx.inner.signer(),
        to: tx.to(),
        contract_created: receipt.contract_address,
        value: tx.value(),
        input: tx.input().clone(),
        nonce: tx.nonce(),
        gas_limit: tx.gas_limit(),
        gas_used: receipt.gas_used,
        effective_gas_price: receipt.effective_gas_price,
        status: receipt.status(),
        tx_type: tx.ty(),
        block_number,
        block_timestamp: Some(block.header.timestamp),
        transaction_index: receipt.transaction_index,
        base_fee_per_gas: block.header.base_fee_per_gas,
    };
    let receipt_logs = receipt
        .logs()
        .iter()
        .map(|l| ReceiptLog {
            address: l.address(),
            topics: l.topics().to_vec(),
            data: l.data().data.clone(),
            log_index: l.log_index,
        })
        .collect();

    Ok(TxMeta { chain_id, summary, coinbase: Some(block.header.beneficiary), receipt_logs })
}

fn call_tracer_options(with_log: bool) -> GethDebugTracingOptions {
    let config = CallConfig { only_top_call: Some(false), with_log: Some(with_log) };
    GethDebugTracingOptions {
        tracer: Some(GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::CallTracer)),
        tracer_config: GethDebugTracerConfig(
            serde_json::to_value(config).expect("static config serializes"),
        ),
        ..Default::default()
    }
}

fn prestate_options() -> GethDebugTracingOptions {
    let config = PreStateConfig { diff_mode: Some(true), ..Default::default() };
    GethDebugTracingOptions {
        tracer: Some(GethDebugTracerType::BuiltInTracer(
            GethDebugBuiltInTracerType::PreStateTracer,
        )),
        tracer_config: GethDebugTracerConfig(
            serde_json::to_value(config).expect("static config serializes"),
        ),
        ..Default::default()
    }
}

fn struct_log_options() -> GethDebugTracingOptions {
    GethDebugTracingOptions {
        config: GethDefaultTracingOptions {
            enable_memory: Some(true),
            disable_memory: Some(false),
            disable_stack: Some(false),
            disable_storage: Some(false),
            enable_return_data: Some(true),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Run `callTracer` (with `withLog`). Distinguishes "this endpoint has no
/// debug API" from other failures so `auto` mode can fall back to a fork.
pub async fn call_tracer_frame(
    provider: &DynProvider,
    hash: B256,
) -> Result<alloy::rpc::types::trace::geth::CallFrame, ClientError> {
    match provider.debug_trace_transaction(hash, call_tracer_options(true)).await {
        Ok(GethTrace::CallTracer(frame)) => Ok(frame),
        Ok(_) => Err(ClientError::Rpc("unexpected callTracer response shape".into())),
        Err(e) => {
            let msg = e.to_string();
            if is_unsupported_error(&msg) {
                Err(ClientError::DebugUnsupported(msg))
            } else {
                Err(ClientError::Rpc(msg))
            }
        }
    }
}

/// Run `prestateTracer` in diff mode. Optional: `None` (with a debug log)
/// when the endpoint can't serve it — balance changes then derive from
/// transfers instead.
pub async fn prestate_diff(provider: &DynProvider, hash: B256) -> Option<DiffMode> {
    match provider.debug_trace_transaction(hash, prestate_options()).await {
        Ok(GethTrace::PreStateTracer(PreStateFrame::Diff(diff))) => Some(diff),
        Ok(other) => {
            tracing::debug!(?other, "unexpected prestateTracer response shape");
            None
        }
        Err(e) => {
            tracing::debug!(error = %e, "prestateTracer unavailable");
            None
        }
    }
}

/// Run the default (struct-log) tracer with memory capture. Optional —
/// payloads are large and some endpoints refuse them.
pub async fn struct_logs(provider: &DynProvider, hash: B256) -> Option<DefaultFrame> {
    match provider.debug_trace_transaction(hash, struct_log_options()).await {
        Ok(GethTrace::Default(frame)) => Some(frame),
        Ok(other) => {
            tracing::debug!(?other, "unexpected struct-log response shape");
            None
        }
        Err(e) => {
            tracing::debug!(error = %e, "struct-log trace unavailable");
            None
        }
    }
}
