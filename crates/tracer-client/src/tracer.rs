//! The orchestrator: resolve a backend, gather artifacts, normalize,
//! analyze, enrich, and assemble the final [`TraceReport`].

use crate::{
    ClientError, anvil,
    anvil::ForkSession,
    artifacts::{self, TxMeta},
    enrich, rpc,
};
use alloy::{providers::DynProvider, rpc::types::trace::geth::DiffMode};
use alloy_primitives::{Address, B256, U256};
use std::collections::{BTreeMap, HashSet};
use tracer_analysis::{
    BalanceInput, TransferContext, balance::NativeDiff, build_fund_flow, collect_transfers,
    compute_balance_changes, fundflow::FundFlowContext, transfers::contract_addresses,
};
use tracer_core::{
    Asset, BackendInfo, BackendKind, Frame, SCHEMA_VERSION, TokenInfo, TokenStandard, TraceReport,
    labels,
};
use tracer_trace::{
    assign_log_indices, interpret_struct_logs, merge_deep_into, normalize_call_frame,
    structlog::RootCall,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackendChoice {
    /// Try the endpoint's debug API; fall back to an anvil fork.
    #[default]
    Auto,
    /// Require `debug_traceTransaction` on the endpoint.
    Rpc,
    /// Always re-execute on a local anvil fork.
    AnvilFork,
}

#[derive(Clone, Debug)]
pub struct TracerConfig {
    pub rpc_url: String,
    pub backend: BackendChoice,
    /// Also run the struct-log interpreter (storage accesses, exact event
    /// interleaving) and merge it into the call tree.
    pub deep: bool,
    /// Replay preceding same-block transactions in fork mode.
    pub replay: bool,
    /// Fetch token metadata (symbol/decimals) and format amounts.
    pub enrich: bool,
    /// Fold gas fees into derived balance changes.
    pub include_gas: bool,
    /// Override the canonical wrapped-native token address.
    pub wrapped_native: Option<Address>,
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self {
            rpc_url: String::new(),
            backend: BackendChoice::Auto,
            deep: false,
            replay: true,
            enrich: true,
            include_gas: true,
            wrapped_native: None,
        }
    }
}

pub struct Tracer {
    cfg: TracerConfig,
    remote: DynProvider,
}

/// Cap on token metadata lookups per report.
const MAX_ENRICH: usize = 50;

impl Tracer {
    pub fn connect(cfg: TracerConfig) -> Result<Self, ClientError> {
        let remote = rpc::connect(&cfg.rpc_url)?;
        Ok(Self { cfg, remote })
    }

    /// Produce the full report for a mined transaction.
    pub async fn report(&self, hash: B256) -> Result<TraceReport, ClientError> {
        let mut warnings: Vec<String> = Vec::new();
        let meta = artifacts::fetch_meta(&self.remote, hash).await?;

        // Resolve the backend; `session` keeps anvil alive until we are done.
        let mut session: Option<ForkSession> = None;
        let (frame, trace_provider, traced_hash, backend_kind) = match self.cfg.backend {
            BackendChoice::Rpc => {
                let f = artifacts::call_tracer_frame(&self.remote, hash).await?;
                (f, self.remote.clone(), hash, BackendKind::Rpc)
            }
            BackendChoice::AnvilFork => {
                let (s, f) = self.fork_backend(&meta, &mut warnings).await?;
                let out = (f, s.provider.clone(), s.traced_hash, BackendKind::AnvilFork);
                session = Some(s);
                out
            }
            BackendChoice::Auto => match artifacts::call_tracer_frame(&self.remote, hash).await {
                Ok(f) => (f, self.remote.clone(), hash, BackendKind::Rpc),
                Err(ClientError::DebugUnsupported(msg)) => {
                    warnings.push(format!(
                        "endpoint lacks debug_traceTransaction ({msg}); \
                         falling back to a local anvil fork"
                    ));
                    let (s, f) = self.fork_backend(&meta, &mut warnings).await?;
                    let out = (f, s.provider.clone(), s.traced_hash, BackendKind::AnvilFork);
                    session = Some(s);
                    out
                }
                Err(e) => return Err(e),
            },
        };
        let fork_info = session.as_ref().map(|s| s.fork.clone());

        // Normalize the call tree.
        let (mut root, norm_warnings) = normalize_call_frame(&frame);
        warnings.extend(norm_warnings);

        // Optional prestate diff for exact native balances.
        let prestate = artifacts::prestate_diff(&trace_provider, traced_hash).await;
        if prestate.is_none() {
            warnings.push(
                "prestateTracer unavailable; native balance changes are derived from \
                 transfers and gas"
                    .into(),
            );
        }

        // Optional deep pass.
        if self.cfg.deep {
            match artifacts::struct_logs(&trace_provider, traced_hash).await {
                // Anvil without `--steps-tracing` answers with an empty
                // stream; treat that as unavailable rather than mis-merging.
                Some(df) if df.struct_logs.is_empty() && !root.children.is_empty() => {
                    warnings.push(
                        "endpoint returned empty struct logs; if it is anvil, restart it \
                         with --steps-tracing to enable --deep data"
                            .into(),
                    );
                }
                Some(df) => {
                    let rc = RootCall {
                        from: meta.summary.from,
                        to: meta.summary.to.or(meta.summary.contract_created),
                        create: meta.summary.to.is_none(),
                        value: meta.summary.value,
                        input: meta.summary.input.clone(),
                        gas_limit: meta.summary.gas_limit,
                    };
                    match interpret_struct_logs(&df, &rc) {
                        Ok(outcome) => {
                            warnings.extend(outcome.warnings);
                            if !merge_deep_into(&mut root, &outcome.root) {
                                warnings.push(
                                    "deep trace did not align with callTracer output; \
                                     storage data omitted"
                                        .into(),
                                );
                            }
                        }
                        Err(e) => warnings.push(format!("struct-log interpretation failed: {e}")),
                    }
                }
                None => warnings
                    .push("struct logs unavailable from this backend; --deep data omitted".into()),
            }
        }

        // Receipt log indices.
        if !meta.receipt_logs.is_empty() && !assign_log_indices(&mut root, &meta.receipt_logs) {
            warnings.push(
                "trace logs did not line up with receipt logs; log indices not assigned".into(),
            );
        }

        // Transfers → balance changes → fund flow.
        let wrapped = self.cfg.wrapped_native.or_else(|| labels::wrapped_native(meta.chain_id));
        let tctx =
            TransferContext { receipt_logs: Some(&meta.receipt_logs), wrapped_native: wrapped };
        let (mut transfers, twarn) = collect_transfers(&root, &tctx);
        warnings.extend(twarn);

        let native_diff = prestate.as_ref().map(native_diff_from);
        let binput = BalanceInput {
            transfers: &transfers,
            tx_from: meta.summary.from,
            coinbase: meta.coinbase,
            gas_used: meta.summary.gas_used,
            effective_gas_price: meta.summary.effective_gas_price,
            base_fee_per_gas: meta.summary.base_fee_per_gas,
            prestate_native: native_diff.as_ref(),
            include_gas: self.cfg.include_gas,
        };
        let mut balance_changes = compute_balance_changes(&binput);

        let contracts = contract_addresses(&root);
        let mut fund_flow =
            build_fund_flow(&transfers, &FundFlowContext { tx_from: meta.summary.from, contracts });

        // Token metadata + amount formatting.
        let mut tokens: BTreeMap<Address, TokenInfo> = BTreeMap::new();
        for t in &transfers {
            if let Some(addr) = t.asset.token() {
                tokens.entry(addr).or_insert_with(|| TokenInfo {
                    address: addr,
                    standard: standard_of(&t.asset),
                    symbol: None,
                    name: None,
                    decimals: None,
                });
            }
        }
        if self.cfg.enrich {
            if tokens.len() > MAX_ENRICH {
                warnings.push(format!(
                    "{} tokens involved; metadata fetched for the first {MAX_ENRICH}",
                    tokens.len()
                ));
            }
            for info in tokens.values_mut().take(MAX_ENRICH) {
                enrich::fill_token_info(&self.remote, info).await;
            }
        }
        let decimals_for = |asset: &Asset| -> Option<u8> {
            match asset {
                Asset::Native => Some(18),
                Asset::Erc20 { token } => tokens.get(token).and_then(|t| t.decimals),
                _ => None,
            }
        };
        for t in &mut transfers {
            if let Some(d) = decimals_for(&t.asset) {
                t.amount.format_with(d);
            }
        }
        for c in &mut balance_changes.changes {
            if let Some(n) = &mut c.native {
                n.delta.format_with(18);
                if let Some(g) = &mut n.gas_fee {
                    g.format_with(18);
                }
            }
            for tc in &mut c.tokens {
                if let Some(d) = decimals_for(&tc.asset) {
                    tc.delta.format_with(d);
                }
            }
        }
        for e in &mut fund_flow.edges {
            if let Some(d) = decimals_for(&e.asset) {
                e.amount.format_with(d);
            }
        }

        // Labels: built-ins plus token symbols.
        let mut address_labels: BTreeMap<Address, String> = BTreeMap::new();
        for addr in involved_addresses(&root, &transfers) {
            if let Some(l) = labels::label(meta.chain_id, addr) {
                address_labels.insert(addr, l.to_string());
            }
        }
        for (addr, info) in &tokens {
            if let Some(symbol) = &info.symbol {
                address_labels.entry(*addr).or_insert_with(|| symbol.clone());
            }
        }
        for c in &mut balance_changes.changes {
            c.label = address_labels.get(&c.address).cloned();
        }
        for n in &mut fund_flow.nodes {
            n.label = address_labels.get(&n.id).cloned();
        }

        let report = TraceReport {
            schema_version: SCHEMA_VERSION.into(),
            chain_id: meta.chain_id,
            native_symbol: labels::native_symbol(meta.chain_id).into(),
            tx: meta.summary,
            backend: BackendInfo {
                kind: backend_kind,
                endpoint_host: rpc::redact_endpoint(&self.cfg.rpc_url),
                fork: fork_info,
            },
            trace: Some(root),
            transfers,
            balance_changes: Some(balance_changes),
            fund_flow: Some(fund_flow),
            tokens,
            address_labels,
            warnings,
        };
        drop(session); // shut anvil down explicitly once everything is gathered
        Ok(report)
    }

    async fn fork_backend(
        &self,
        meta: &TxMeta,
        warnings: &mut Vec<String>,
    ) -> Result<(ForkSession, alloy::rpc::types::trace::geth::CallFrame), ClientError> {
        let session =
            anvil::fork_and_replay(&self.cfg.rpc_url, &self.remote, meta, self.cfg.replay).await?;
        warnings.extend(session.warnings.iter().cloned());
        let frame = artifacts::call_tracer_frame(&session.provider, session.traced_hash).await?;
        Ok((session, frame))
    }
}

fn standard_of(asset: &Asset) -> TokenStandard {
    match asset {
        Asset::Native => TokenStandard::Unknown,
        Asset::Erc20 { .. } => TokenStandard::Erc20,
        Asset::Erc721 { .. } => TokenStandard::Erc721,
        Asset::Erc1155 { .. } => TokenStandard::Erc1155,
    }
}

/// `prestateTracer` diff → native balance pre/post pairs. Accounts present
/// only in `pre` were destroyed (post balance zero); `post` carries only
/// changed fields.
fn native_diff_from(diff: &DiffMode) -> NativeDiff {
    let mut out = NativeDiff::new();
    for (addr, post_state) in &diff.post {
        if let Some(post_balance) = post_state.balance {
            let pre_balance = diff.pre.get(addr).and_then(|s| s.balance);
            out.insert(*addr, (pre_balance, Some(post_balance)));
        }
    }
    for (addr, pre_state) in &diff.pre {
        if !diff.post.contains_key(addr)
            && let Some(pre_balance) = pre_state.balance
            && !pre_balance.is_zero()
        {
            out.insert(*addr, (Some(pre_balance), Some(U256::ZERO)));
        }
    }
    out
}

fn involved_addresses(root: &Frame, transfers: &[tracer_core::AssetTransfer]) -> HashSet<Address> {
    let mut set = HashSet::new();
    for f in root.iter() {
        set.insert(f.from);
        if let Some(to) = f.to {
            set.insert(to);
        }
    }
    for t in transfers {
        set.insert(t.from);
        set.insert(t.to);
        if let Some(token) = t.asset.token() {
            set.insert(token);
        }
    }
    set
}
