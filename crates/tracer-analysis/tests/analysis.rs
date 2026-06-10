//! Analyzer tests over a synthetic trace mirroring a realistic transaction:
//! ETH in, an internal send, a token transfer, and a reverted subtree whose
//! inner (successful) transfer must not count.

use alloy_primitives::{Address, B256, Bytes, U256, address, b256};
use std::collections::BTreeMap;
use tracer_analysis::{
    BalanceInput, TransferContext, balance::NativeDiff, build_fund_flow, collect_transfers,
    compute_balance_changes, fundflow::FundFlowContext, transfers::contract_addresses,
};
use tracer_core::{
    Amount, Asset, AssetTransfer, CallKind, Frame, FrameLog, NativeSource, NodeKind, ReceiptLog,
    TransferOrigin,
};

const EOA0: Address = address!("0x1000000000000000000000000000000000000001");
const MAIN: Address = address!("0x2000000000000000000000000000000000000002");
const RECIP: Address = address!("0x3000000000000000000000000000000000000003");
const TOKEN: Address = address!("0x00000000000000000000000000000000000a11ce");
const REVERTER: Address = address!("0x6000000000000000000000000000000000000006");
const EOA1: Address = address!("0x7000000000000000000000000000000000000007");
const EOA2: Address = address!("0x8000000000000000000000000000000000000008");
const COINBASE: Address = address!("0xc00000000000000000000000000000000000000c");

const TRANSFER_SIG: B256 =
    b256!("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");

fn transfer_log(from: Address, to: Address, value: u64, position: Option<u64>) -> FrameLog {
    FrameLog {
        address: TOKEN,
        topics: vec![TRANSFER_SIG, from.into_word(), to.into_word()],
        data: Bytes::from(U256::from(value).to_be_bytes::<32>().to_vec()),
        position,
        log_index: None,
        decoded: None,
    }
}

/// root: EOA0 -CALL(1000)-> MAIN
///   ├─ CALL MAIN -> EOA1, value 1
///   ├─ CALL MAIN -> TOKEN, emits Transfer(MAIN, RECIP, 5)
///   └─ CALL MAIN -> REVERTER (reverted)
///        └─ CALL REVERTER -> EOA2, value 5 (succeeded, but parent reverted)
fn tree(with_frame_logs: bool) -> Frame {
    let mut root = Frame::new(CallKind::Call, EOA0);
    root.to = Some(MAIN);
    root.value = U256::from(1000u64);

    let mut send = Frame::new(CallKind::Call, MAIN);
    send.to = Some(EOA1);
    send.value = U256::from(1u8);

    let mut token_call = Frame::new(CallKind::Call, MAIN);
    token_call.to = Some(TOKEN);
    token_call.input = Bytes::from(vec![0xa9, 0x05, 0x9c, 0xbb]);
    token_call.output = Bytes::from(U256::ONE.to_be_bytes::<32>().to_vec());
    if with_frame_logs {
        token_call.logs.push(transfer_log(MAIN, RECIP, 5, Some(0)));
    }

    let mut inner_send = Frame::new(CallKind::Call, REVERTER);
    inner_send.to = Some(EOA2);
    inner_send.value = U256::from(5u8);

    let mut reverter = Frame::new(CallKind::Call, MAIN);
    reverter.to = Some(REVERTER);
    reverter.error = Some("execution reverted".into());
    reverter.children.push(inner_send);

    root.children.push(send);
    root.children.push(token_call);
    root.children.push(reverter);
    root.assign_ids();
    root
}

#[test]
fn collects_ordered_transfers_and_excludes_reverted() {
    let root = tree(true);
    let (transfers, warnings) = collect_transfers(&root, &TransferContext::default());
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(transfers.len(), 3, "{transfers:#?}");

    assert_eq!(transfers[0].origin, TransferOrigin::TxValue);
    assert_eq!(transfers[0].amount.raw, U256::from(1000u64));
    assert_eq!((transfers[0].from, transfers[0].to), (EOA0, MAIN));

    assert!(matches!(transfers[1].origin, TransferOrigin::Call { .. }));
    assert_eq!((transfers[1].from, transfers[1].to), (MAIN, EOA1));

    assert_eq!(transfers[2].asset, Asset::Erc20 { token: TOKEN });
    assert_eq!((transfers[2].from, transfers[2].to), (MAIN, RECIP));
    assert_eq!(transfers[2].amount.raw, U256::from(5u8));

    let orders: Vec<u32> = transfers.iter().map(|t| t.order).collect();
    assert_eq!(orders, vec![0, 1, 2]);

    // The 5-wei transfer inside the reverted subtree must not appear.
    assert!(!transfers.iter().any(|t| t.to == EOA2));
}

#[test]
fn falls_back_to_receipt_logs() {
    let root = tree(false);
    let receipt_logs = vec![ReceiptLog {
        address: TOKEN,
        topics: vec![TRANSFER_SIG, MAIN.into_word(), RECIP.into_word()],
        data: Bytes::from(U256::from(5u8).to_be_bytes::<32>().to_vec()),
        log_index: Some(7),
    }];
    let ctx = TransferContext { receipt_logs: Some(&receipt_logs), wrapped_native: None };
    let (transfers, warnings) = collect_transfers(&root, &ctx);
    assert_eq!(transfers.len(), 3);
    assert_eq!(transfers[2].origin, TransferOrigin::Log { log_index: Some(7) });
    assert!(warnings.iter().any(|w| w.contains("receipt log order")));
}

#[test]
fn wrap_events_only_honored_for_canonical_wrapper() {
    let weth = address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
    let deposit_sig = b256!("0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c");
    let mut root = Frame::new(CallKind::Call, EOA0);
    root.to = Some(weth);
    root.value = U256::from(100u8);
    root.logs.push(FrameLog {
        address: weth,
        topics: vec![deposit_sig, EOA0.into_word()],
        data: Bytes::from(U256::from(100u8).to_be_bytes::<32>().to_vec()),
        position: Some(0),
        log_index: None,
        decoded: None,
    });
    root.assign_ids();

    let ctx = TransferContext { receipt_logs: None, wrapped_native: Some(weth) };
    let (transfers, _) = collect_transfers(&root, &ctx);
    assert_eq!(transfers.len(), 2);
    assert_eq!(transfers[1].origin, TransferOrigin::Deposit { log_index: None });
    assert_eq!((transfers[1].from, transfers[1].to), (weth, EOA0));
    assert_eq!(transfers[1].asset, Asset::Erc20 { token: weth });

    // Without the canonical wrapper configured, the event is ignored.
    let (transfers, _) = collect_transfers(&root, &TransferContext::default());
    assert_eq!(transfers.len(), 1);
}

#[test]
fn derived_balance_changes_account_for_gas() {
    let root = tree(true);
    let (transfers, _) = collect_transfers(&root, &TransferContext::default());
    let input = BalanceInput {
        transfers: &transfers,
        tx_from: EOA0,
        coinbase: Some(COINBASE),
        gas_used: 1000,
        effective_gas_price: 3,
        base_fee_per_gas: Some(1),
        prestate_native: None,
        include_gas: true,
    };
    let bc = compute_balance_changes(&input);
    assert_eq!(bc.native_source, NativeSource::Derived);
    assert!(bc.gas_included);

    let by_addr: BTreeMap<Address, _> = bc.changes.iter().map(|c| (c.address, c)).collect();

    // Sender row comes first: -1000 value - 3000 gas.
    assert_eq!(bc.changes[0].address, EOA0);
    let sender = &by_addr[&EOA0].native.as_ref().unwrap();
    assert_eq!(sender.delta.dec, "-4000");
    assert_eq!(sender.gas_fee.as_ref().unwrap().raw, U256::from(3000u64));

    let main = &by_addr[&MAIN];
    assert_eq!(main.native.as_ref().unwrap().delta.dec, "999");
    assert_eq!(main.tokens.len(), 1);
    assert_eq!(main.tokens[0].delta.dec, "-5");

    assert_eq!(by_addr[&EOA1].native.as_ref().unwrap().delta.dec, "1");
    assert_eq!(by_addr[&RECIP].tokens[0].delta.dec, "5");
    assert_eq!(by_addr[&COINBASE].native.as_ref().unwrap().delta.dec, "2000");

    // Conservation: native deltas (excluding burned base fee) sum to -base burn.
    // -4000 + 999 + 1 + 2000 = -1000 = -(base fee burn).
    assert!(!by_addr.contains_key(&EOA2));
}

#[test]
fn prestate_balance_changes_are_exact() {
    let root = tree(true);
    let (transfers, _) = collect_transfers(&root, &TransferContext::default());
    let mut diff = NativeDiff::new();
    diff.insert(EOA0, (Some(U256::from(10_000u64)), Some(U256::from(6_000u64))));
    diff.insert(MAIN, (Some(U256::ZERO), Some(U256::from(999u64))));
    diff.insert(EOA1, (Some(U256::from(5u8)), Some(U256::from(6u8))));
    diff.insert(COINBASE, (None, Some(U256::from(2000u64))));

    let input = BalanceInput {
        transfers: &transfers,
        tx_from: EOA0,
        coinbase: Some(COINBASE),
        gas_used: 1000,
        effective_gas_price: 3,
        base_fee_per_gas: Some(1),
        prestate_native: Some(&diff),
        include_gas: true,
    };
    let bc = compute_balance_changes(&input);
    assert_eq!(bc.native_source, NativeSource::Prestate);

    let by_addr: BTreeMap<Address, _> = bc.changes.iter().map(|c| (c.address, c)).collect();
    let sender = by_addr[&EOA0].native.as_ref().unwrap();
    assert_eq!(sender.delta.dec, "-4000");
    assert_eq!(sender.pre, Some(U256::from(10_000u64)));
    assert_eq!(sender.post, Some(U256::from(6_000u64)));
    assert_eq!(by_addr[&COINBASE].native.as_ref().unwrap().delta.dec, "2000");
    // Token rows still come from transfers.
    assert_eq!(by_addr[&RECIP].tokens[0].delta.dec, "5");
}

#[test]
fn fund_flow_nodes_and_edges() {
    let root = tree(true);
    let (transfers, _) = collect_transfers(&root, &TransferContext::default());
    let contracts = contract_addresses(&root);
    assert!(contracts.contains(&MAIN));
    assert!(contracts.contains(&TOKEN));
    assert!(contracts.contains(&REVERTER));
    assert!(!contracts.contains(&EOA1));

    let flow = build_fund_flow(&transfers, &FundFlowContext { tx_from: EOA0, contracts });
    assert_eq!(flow.edges.len(), 3);
    assert_eq!(flow.nodes.len(), 4); // EOA0, MAIN, EOA1, RECIP

    let kind_of = |a: Address| flow.nodes.iter().find(|n| n.id == a).map(|n| n.kind);
    assert_eq!(kind_of(EOA0), Some(NodeKind::Eoa));
    assert_eq!(kind_of(MAIN), Some(NodeKind::Contract));
    assert_eq!(kind_of(EOA1), Some(NodeKind::Account));
    assert_eq!(kind_of(RECIP), Some(NodeKind::Account));

    // Token contract becomes a party (and a Token node) in wrap flows.
    let wrap = vec![AssetTransfer {
        order: 0,
        from: TOKEN,
        to: EOA0,
        asset: Asset::Erc20 { token: TOKEN },
        amount: Amount::new(U256::from(1u8)),
        origin: TransferOrigin::Deposit { log_index: None },
    }];
    let flow =
        build_fund_flow(&wrap, &FundFlowContext { tx_from: EOA0, contracts: Default::default() });
    assert_eq!(flow.nodes.iter().find(|n| n.id == TOKEN).unwrap().kind, NodeKind::Token);
}
