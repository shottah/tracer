//! Extract ordered asset transfers from a normalized trace.
//!
//! Native flows come from the call tree (tx value, `CALL` value, `CREATE`
//! endowments, `SELFDESTRUCT` sweeps); token flows come from
//! ERC-20/721/1155 and wrapped-native events. Transfers inside reverted
//! subtrees are excluded. When frame logs carry `position` data, calls and
//! events interleave into exact execution order; otherwise token transfers
//! fall back to receipt order.

use alloy_primitives::{Address, U256};
use std::collections::HashSet;
use tracer_core::{
    Amount, Asset, AssetTransfer, Frame, FrameLog, ReceiptLog, TransferOrigin,
    decode::{TokenEvent, decode_token_event},
};

#[derive(Clone, Debug, Default)]
pub struct TransferContext<'a> {
    /// Receipt logs (logIndex order); used as a fallback when the trace
    /// carries no event data, and to stamp log indices.
    pub receipt_logs: Option<&'a [ReceiptLog]>,
    /// The chain's canonical wrapped-native token. `Deposit`/`Withdrawal`
    /// events are treated as wrap/unwrap flows only for this address.
    pub wrapped_native: Option<Address>,
}

/// Collect transfers in execution order. Returns transfers plus warnings.
pub fn collect_transfers(
    root: &Frame,
    ctx: &TransferContext<'_>,
) -> (Vec<AssetTransfer>, Vec<String>) {
    let mut out: Vec<AssetTransfer> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    let frames_have_logs = root.iter().any(|f| !f.logs.is_empty());
    let receipt_has_logs = ctx.receipt_logs.is_some_and(|l| !l.is_empty());
    let use_frame_logs = frames_have_logs || !receipt_has_logs;

    walk(root, true, use_frame_logs, ctx, &mut out);

    if !use_frame_logs && receipt_has_logs {
        let mut emitted = false;
        for rl in ctx.receipt_logs.unwrap() {
            emitted |= emit_token_log(
                rl.address,
                &rl.topics,
                rl.data.as_ref(),
                rl.log_index,
                ctx,
                &mut out,
            );
        }
        if emitted && root.iter().any(|f| !f.children.is_empty()) {
            warnings.push(
                "trace carried no event data; token transfer ordering falls back to \
                 receipt log order"
                    .into(),
            );
        }
    }

    for (i, t) in out.iter_mut().enumerate() {
        t.order = i as u32;
    }
    (out, warnings)
}

fn walk(
    f: &Frame,
    parent_effective: bool,
    use_frame_logs: bool,
    ctx: &TransferContext<'_>,
    out: &mut Vec<AssetTransfer>,
) {
    let effective = parent_effective && f.ok();

    if effective
        && f.kind.moves_value()
        && !f.value.is_zero()
        && let Some(to) = f.to
    {
        let origin = match f.kind {
            k if k.is_create() => TransferOrigin::Create { frame_id: f.id },
            tracer_core::CallKind::SelfDestruct => TransferOrigin::SelfDestruct { frame_id: f.id },
            _ if f.parent.is_none() => TransferOrigin::TxValue,
            _ => TransferOrigin::Call { frame_id: f.id },
        };
        out.push(AssetTransfer {
            order: 0,
            from: f.from,
            to,
            asset: Asset::Native,
            amount: Amount::new(f.value),
            origin,
        });
    }

    if use_frame_logs {
        let mut li = 0;
        let flush = |upto: Option<usize>, li: &mut usize, out: &mut Vec<AssetTransfer>| {
            while *li < f.logs.len() {
                let log = &f.logs[*li];
                let due = match (upto, log.position) {
                    (Some(ci), Some(p)) => p <= ci as u64,
                    (Some(_), None) => false,
                    (None, _) => true,
                };
                if !due {
                    break;
                }
                if effective {
                    emit_frame_log(log, ctx, out);
                }
                *li += 1;
            }
        };
        for (ci, c) in f.children.iter().enumerate() {
            flush(Some(ci), &mut li, out);
            walk(c, effective, use_frame_logs, ctx, out);
        }
        flush(None, &mut li, out);
    } else {
        for c in &f.children {
            walk(c, effective, use_frame_logs, ctx, out);
        }
    }
}

fn emit_frame_log(log: &FrameLog, ctx: &TransferContext<'_>, out: &mut Vec<AssetTransfer>) {
    emit_token_log(log.address, &log.topics, log.data.as_ref(), log.log_index, ctx, out);
}

fn emit_token_log(
    address: Address,
    topics: &[alloy_primitives::B256],
    data: &[u8],
    log_index: Option<u64>,
    ctx: &TransferContext<'_>,
    out: &mut Vec<AssetTransfer>,
) -> bool {
    let Some(ev) = decode_token_event(address, topics, data) else { return false };
    let origin = TransferOrigin::Log { log_index };
    match ev {
        TokenEvent::Erc20Transfer { token, from, to, value } => out.push(AssetTransfer {
            order: 0,
            from,
            to,
            asset: Asset::Erc20 { token },
            amount: Amount::new(value),
            origin,
        }),
        TokenEvent::Erc721Transfer { token, from, to, token_id } => out.push(AssetTransfer {
            order: 0,
            from,
            to,
            asset: Asset::Erc721 { token, token_id },
            amount: Amount::new(U256::ONE),
            origin,
        }),
        TokenEvent::Erc1155Single { token, from, to, token_id, value } => {
            out.push(AssetTransfer {
                order: 0,
                from,
                to,
                asset: Asset::Erc1155 { token, token_id },
                amount: Amount::new(value),
                origin,
            });
        }
        TokenEvent::Erc1155Batch { token, from, to, ids, values } => {
            for (id, value) in ids.into_iter().zip(values) {
                out.push(AssetTransfer {
                    order: 0,
                    from,
                    to,
                    asset: Asset::Erc1155 { token, token_id: id },
                    amount: Amount::new(value),
                    origin,
                });
            }
        }
        TokenEvent::Deposit { token, dst, wad } => {
            // Wrap: the wrapper mints its token to `dst`. Only honored for
            // the chain's canonical wrapped-native contract.
            if ctx.wrapped_native == Some(token) {
                out.push(AssetTransfer {
                    order: 0,
                    from: token,
                    to: dst,
                    asset: Asset::Erc20 { token },
                    amount: Amount::new(wad),
                    origin: TransferOrigin::Deposit { log_index },
                });
            } else {
                return false;
            }
        }
        TokenEvent::Withdrawal { token, src, wad } => {
            if ctx.wrapped_native == Some(token) {
                out.push(AssetTransfer {
                    order: 0,
                    from: src,
                    to: token,
                    asset: Asset::Erc20 { token },
                    amount: Amount::new(wad),
                    origin: TransferOrigin::Withdrawal { log_index },
                });
            } else {
                return false;
            }
        }
    }
    true
}

/// Addresses that demonstrably have code, judged from the trace: call targets
/// that executed something (input, children, or logs) plus created contracts.
pub fn contract_addresses(root: &Frame) -> HashSet<Address> {
    let mut set = HashSet::new();
    for f in root.iter() {
        let executed = !f.input.is_empty()
            || !f.children.is_empty()
            || !f.logs.is_empty()
            || !f.output.is_empty();
        if f.kind == tracer_core::CallKind::SelfDestruct {
            // The self-destructing context has code; the beneficiary may not.
            set.insert(f.from);
            continue;
        }
        if let Some(to) = f.to
            && (executed || f.kind.is_create())
        {
            set.insert(to);
        }
    }
    set
}
