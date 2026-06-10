//! Synthetic struct-log streams exercising the interpreter's tricky paths.

use alloy_primitives::{Address, B256, Bytes, U256, address, hex};
use alloy_rpc_types_trace::geth::DefaultFrame;
use serde_json::{Value, json};
use tracer_core::CallKind;
use tracer_trace::structlog::{RootCall, interpret_struct_logs};

const SENDER: Address = address!("0x1000000000000000000000000000000000000001");
const MAIN: Address = address!("0x2000000000000000000000000000000000000002");
const TOKEN: Address = address!("0x00000000000000000000000000000000000a11ce");
const EOA: Address = address!("0x7000000000000000000000000000000000000007");
const LIB: Address = address!("0x9000000000000000000000000000000000000009");

const TRANSFER_SIG: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

fn slog(depth: u64, op: &str, stack: &[&str], memory: Option<Vec<String>>) -> Value {
    let mut v =
        json!({"pc": 0, "op": op, "gas": 500000u64, "gasCost": 3, "depth": depth, "stack": stack});
    if let Some(m) = memory {
        v["memory"] = json!(m);
    }
    v
}

fn frame(logs: Vec<Value>, failed: bool, ret: &str) -> DefaultFrame {
    serde_json::from_value(
        json!({"gas": 21000u64, "failed": failed, "returnValue": ret, "structLogs": logs}),
    )
    .expect("DefaultFrame parses")
}

/// Chunk a hex string (no 0x) into 32-byte memory words, zero-padded.
fn words(hexstr: &str) -> Vec<String> {
    let s = hexstr.trim_start_matches("0x");
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let end = (i + 64).min(s.len());
        let mut w = s[i..end].to_string();
        while w.len() < 64 {
            w.push('0');
        }
        out.push(w);
        i = end;
    }
    out
}

fn pad_addr(a: Address) -> String {
    format!("0x{}", hex::encode(a.into_word()))
}

fn root_call(value: u64) -> RootCall {
    RootCall {
        from: SENDER,
        to: Some(MAIN),
        create: false,
        value: U256::from(value),
        input: Bytes::new(),
        gas_limit: 1_000_000,
    }
}

#[test]
fn nested_call_with_log_and_return() {
    let calldata = "a9059cbb\
                    0000000000000000000000003000000000000000000000000000000000000003\
                    0000000000000000000000000000000000000000000000000000000000000005";
    let amount_word = "0000000000000000000000000000000000000000000000000000000000000005";
    let ret_word = "0000000000000000000000000000000000000000000000000000000000000001";

    let logs = vec![
        // CALL(gas, TOKEN, 0, 0, 0x44, 0, 0x20) — stack listed bottom-first.
        slog(
            1,
            "CALL",
            &["0x20", "0x0", "0x44", "0x0", "0x0", &pad_addr(TOKEN), "0x186a0"],
            Some(words(calldata)),
        ),
        // LOG3(0, 0x20, sig, from=MAIN, to=RECIP)
        slog(
            2,
            "LOG3",
            &[
                "0x0000000000000000000000003000000000000000000000000000000000000003",
                &pad_addr(MAIN),
                TRANSFER_SIG,
                "0x20",
                "0x0",
            ],
            Some(words(amount_word)),
        ),
        slog(2, "RETURN", &["0x20", "0x0"], Some(words(ret_word))),
        slog(1, "POP", &["0x1"], None),
        slog(1, "STOP", &[], None),
    ];
    let df = frame(logs, false, "0x");
    let out = interpret_struct_logs(&df, &root_call(1000)).unwrap();
    let root = &out.root;

    assert_eq!(root.kind, CallKind::Call);
    assert_eq!(root.to, Some(MAIN));
    assert_eq!(root.value, U256::from(1000u64));
    assert_eq!(root.gas_used, 21000);
    assert_eq!(root.children.len(), 1);

    let call = &root.children[0];
    assert_eq!(call.kind, CallKind::Call);
    assert_eq!(call.from, MAIN);
    assert_eq!(call.to, Some(TOKEN));
    assert_eq!(call.input.len(), 0x44);
    assert_eq!(call.decoded.as_ref().unwrap().name.as_deref(), Some("transfer"));
    assert!(call.ok());
    assert_eq!(call.output, Bytes::from(hex::decode(ret_word).unwrap()));

    assert_eq!(call.logs.len(), 1);
    let log = &call.logs[0];
    assert_eq!(log.address, TOKEN);
    assert_eq!(log.topics.len(), 3);
    assert_eq!(log.position, Some(0));
    assert_eq!(log.decoded.as_ref().unwrap().name, "Transfer");
    assert_eq!(U256::from_be_slice(log.data.as_ref()), U256::from(5u8));

    assert_eq!(root.children[0].id, 1);
    assert_eq!(root.children[0].parent, Some(0));
}

#[test]
fn code_less_callee_completes_in_place() {
    let logs = vec![
        slog(1, "CALL", &["0x0", "0x0", "0x0", "0x0", "0x1", &pad_addr(EOA), "0x5208"], None),
        slog(1, "POP", &["0x1"], None),
        slog(1, "STOP", &[], None),
    ];
    let df = frame(logs, false, "0x");
    let out = interpret_struct_logs(&df, &root_call(0)).unwrap();
    assert_eq!(out.root.children.len(), 1);
    let leaf = &out.root.children[0];
    assert_eq!(leaf.kind, CallKind::Call);
    assert_eq!(leaf.to, Some(EOA));
    assert_eq!(leaf.value, U256::from(1u8));
    assert!(leaf.ok());
    assert!(leaf.children.is_empty());
}

#[test]
fn revert_reason_decodes_from_memory() {
    let err = "08c379a0\
               0000000000000000000000000000000000000000000000000000000000000020\
               0000000000000000000000000000000000000000000000000000000000000004\
               6e6f706500000000000000000000000000000000000000000000000000000000";
    let logs = vec![
        slog(1, "CALL", &["0x0", "0x0", "0x0", "0x0", "0x0", &pad_addr(TOKEN), "0x9c40"], None),
        slog(2, "REVERT", &["0x64", "0x0"], Some(words(err))),
        slog(1, "PUSH1", &["0x0"], None),
        slog(1, "STOP", &[], None),
    ];
    let df = frame(logs, false, "0x");
    let out = interpret_struct_logs(&df, &root_call(0)).unwrap();
    let child = &out.root.children[0];
    assert_eq!(child.error.as_deref(), Some("execution reverted"));
    assert_eq!(child.revert_reason.as_deref(), Some("nope"));
    assert_eq!(child.output.len(), 100);
    assert!(out.root.ok());
}

#[test]
fn delegatecall_attributes_to_caller_context() {
    let logs = vec![
        slog(1, "DELEGATECALL", &["0x0", "0x0", "0x0", "0x0", &pad_addr(LIB), "0x9c40"], None),
        slog(2, "SLOAD", &["0x7"], None),
        slog(2, "POP", &["0x0"], None),
        slog(2, "SSTORE", &["0x2a", "0x7"], None),
        slog(2, "SSTORE", &["0x63", "0x7"], None),
        slog(2, "LOG0", &["0x0", "0x0"], None),
        slog(2, "STOP", &[], None),
        slog(1, "POP", &["0x1"], None),
        slog(1, "STOP", &[], None),
    ];
    let df = frame(logs, false, "0x");
    let out = interpret_struct_logs(&df, &root_call(0)).unwrap();
    let child = &out.root.children[0];
    assert_eq!(child.kind, CallKind::DelegateCall);
    assert_eq!(child.from, MAIN);
    assert_eq!(child.to, Some(LIB));
    assert_eq!(child.value, U256::ZERO);

    let slot = B256::from(U256::from(7u8));
    assert_eq!(child.storage_reads.len(), 1);
    assert_eq!(child.storage_reads[0].slot, slot);
    assert_eq!(child.storage_reads[0].value, Some(B256::ZERO));

    assert_eq!(child.storage_writes.len(), 2);
    // The preceding SLOAD seeded the last-known value for this slot.
    assert_eq!(child.storage_writes[0].previous, Some(B256::ZERO));
    assert_eq!(child.storage_writes[0].value, B256::from(U256::from(0x2au8)));
    assert_eq!(child.storage_writes[1].previous, Some(B256::from(U256::from(0x2au8))));
    assert_eq!(child.storage_writes[1].value, B256::from(U256::from(0x63u8)));

    // Logs in a DELEGATECALL frame attribute to the caller's context address.
    assert_eq!(child.logs[0].address, MAIN);
}

#[test]
fn create2_address_precomputed_and_constructor_logs_attributed() {
    let initcode: &[u8] = &[0x00];
    let salt = B256::from(U256::from(0x1234u16));
    let expected = MAIN.create2_from_code(salt, initcode);

    let logs = vec![
        slog(1, "CREATE2", &["0x1234", "0x1", "0x0", "0x0"], Some(words("00"))),
        slog(2, "LOG0", &["0x0", "0x0"], None),
        slog(2, "STOP", &[], None),
        slog(1, "POP", &[&format!("0x{}", hex::encode(expected.into_word()))], None),
        slog(1, "STOP", &[], None),
    ];
    let df = frame(logs, false, "0x");
    let out = interpret_struct_logs(&df, &root_call(0)).unwrap();
    let child = &out.root.children[0];
    assert_eq!(child.kind, CallKind::Create2);
    assert_eq!(child.to, Some(expected));
    assert_eq!(child.input, Bytes::from(initcode.to_vec()));
    assert_eq!(child.logs[0].address, expected);
    assert!(child.ok());
}

#[test]
fn root_revert_is_reported() {
    let err = "08c379a0\
               0000000000000000000000000000000000000000000000000000000000000020\
               0000000000000000000000000000000000000000000000000000000000000004\
               6e6f706500000000000000000000000000000000000000000000000000000000";
    let logs =
        vec![slog(1, "PUSH1", &[], None), slog(1, "REVERT", &["0x64", "0x0"], Some(words(err)))];
    let df = frame(logs, true, &format!("0x{err}"));
    let out = interpret_struct_logs(&df, &root_call(0)).unwrap();
    assert_eq!(out.root.error.as_deref(), Some("execution reverted"));
    assert_eq!(out.root.revert_reason.as_deref(), Some("nope"));
}

#[test]
fn empty_stream_is_a_plain_transfer() {
    let df = frame(vec![], false, "0x");
    let mut rc = root_call(555);
    rc.to = Some(EOA);
    let out = interpret_struct_logs(&df, &rc).unwrap();
    assert!(out.root.children.is_empty());
    assert!(out.root.ok());
    assert_eq!(out.root.value, U256::from(555u16));
}

#[test]
fn missing_memory_degrades_with_warning() {
    let calldata_len = "0x44";
    let logs = vec![
        slog(
            1,
            "CALL",
            &["0x0", "0x0", calldata_len, "0x0", "0x0", &pad_addr(TOKEN), "0x9c40"],
            None,
        ),
        slog(2, "STOP", &[], None),
        slog(1, "POP", &["0x1"], None),
        slog(1, "STOP", &[], None),
    ];
    let df = frame(logs, false, "0x");
    let out = interpret_struct_logs(&df, &root_call(0)).unwrap();
    assert!(out.root.children[0].input.is_empty());
    assert!(out.warnings.iter().any(|w| w.contains("memory")));
}

#[test]
fn parses_realistic_geth_shape() {
    let raw = r#"{
        "gas": 21734,
        "failed": false,
        "returnValue": "0x",
        "structLogs": [
            {"pc": 0, "op": "PUSH1", "gas": 78752, "gasCost": 3, "depth": 1,
             "stack": [], "memory": []},
            {"pc": 2, "op": "MSTORE", "gas": 78749, "gasCost": 12, "depth": 1,
             "stack": ["0x80", "0x40"],
             "memory": ["0000000000000000000000000000000000000000000000000000000000000000"],
             "storage": {}}
        ]
    }"#;
    let df: DefaultFrame = serde_json::from_str(raw).expect("geth-shaped JSON parses");
    let out = interpret_struct_logs(&df, &root_call(0)).unwrap();
    assert!(out.root.children.is_empty());
}
