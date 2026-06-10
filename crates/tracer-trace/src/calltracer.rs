//! Normalization of geth `callTracer` output into the core [`Frame`] model,
//! plus receipt-log index assignment.

use alloy_rpc_types_trace::geth::{CallFrame, CallLogFrame};
use std::collections::HashMap;
use tracer_core::{CallKind, Frame, FrameLog, ReceiptLog, decode};

/// Normalize a geth `callTracer` frame tree. Returns the root frame with DFS
/// ids assigned, and any warnings encountered.
pub fn normalize_call_frame(cf: &CallFrame) -> (Frame, Vec<String>) {
    let mut warnings = Vec::new();
    let mut root = convert(cf, &mut warnings);
    root.assign_ids();
    (root, warnings)
}

fn convert(cf: &CallFrame, warnings: &mut Vec<String>) -> Frame {
    let kind = CallKind::from_geth(&cf.typ).unwrap_or_else(|| {
        warnings.push(format!("unknown callTracer frame type {:?}, treating as CALL", cf.typ));
        CallKind::Call
    });
    let mut f = Frame::new(kind, cf.from);
    f.to = cf.to;
    f.value = cf.value.unwrap_or_default();
    f.gas = u64::try_from(cf.gas).unwrap_or(u64::MAX);
    f.gas_used = u64::try_from(cf.gas_used).unwrap_or(u64::MAX);
    f.input = cf.input.clone();
    f.output = cf.output.clone().unwrap_or_default();
    f.error = cf.error.clone();
    f.revert_reason = cf.revert_reason.clone().or_else(|| decode::revert_reason(f.output.as_ref()));
    if !kind.is_create() {
        f.decoded = decode::decode_call(f.input.as_ref());
    }
    f.logs = cf.logs.iter().filter_map(convert_log).collect();
    f.children = cf.calls.iter().map(|c| convert(c, warnings)).collect();
    f
}

fn convert_log(l: &CallLogFrame) -> Option<FrameLog> {
    let address = l.address?;
    let topics = l.topics.clone().unwrap_or_default();
    let data = l.data.clone().unwrap_or_default();
    let decoded = decode::decode_event(address, &topics, data.as_ref());
    Some(FrameLog { address, topics, data, position: l.position, log_index: None, decoded })
}

/// Walk the tree in execution order (children and logs interleaved via log
/// `position`), yielding `(frame_id, index_of_log_within_frame)`.
pub fn logs_in_execution_order(root: &Frame) -> Vec<(u32, usize)> {
    fn walk(f: &Frame, out: &mut Vec<(u32, usize)>) {
        let mut li = 0;
        for (ci, c) in f.children.iter().enumerate() {
            while li < f.logs.len() && f.logs[li].position.map(|p| p <= ci as u64).unwrap_or(false)
            {
                out.push((f.id, li));
                li += 1;
            }
            walk(c, out);
        }
        while li < f.logs.len() {
            out.push((f.id, li));
            li += 1;
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

/// Match the trace's frame logs (in execution order) against the receipt's
/// logs and stamp `log_index` onto each frame log.
///
/// Returns `false` (leaving the tree untouched) when the sequences disagree —
/// callers should warn and fall back to receipt ordering.
pub fn assign_log_indices(root: &mut Frame, receipt_logs: &[ReceiptLog]) -> bool {
    let order = logs_in_execution_order(root);
    if order.len() != receipt_logs.len() {
        return false;
    }
    // Verify contents line up before mutating anything.
    {
        let by_id: HashMap<u32, &Frame> = root.iter().map(|f| (f.id, f)).collect();
        for ((fid, li), rl) in order.iter().zip(receipt_logs) {
            let fl = &by_id[fid].logs[*li];
            if fl.address != rl.address || fl.topics != rl.topics || fl.data != rl.data {
                return false;
            }
        }
    }
    let assign: HashMap<(u32, usize), Option<u64>> =
        order.iter().zip(receipt_logs).map(|((fid, li), rl)| ((*fid, *li), rl.log_index)).collect();
    fn apply(f: &mut Frame, assign: &HashMap<(u32, usize), Option<u64>>) {
        let id = f.id;
        for (li, log) in f.logs.iter_mut().enumerate() {
            if let Some(idx) = assign.get(&(id, li)) {
                log.log_index = *idx;
            }
        }
        for c in &mut f.children {
            apply(c, assign);
        }
    }
    apply(root, &assign);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Bytes, U256, address, b256};

    const FIXTURE: &str = include_str!("../tests/fixtures/calltracer_nested.json");

    fn fixture() -> CallFrame {
        serde_json::from_str(FIXTURE).expect("fixture parses as geth CallFrame")
    }

    #[test]
    fn normalizes_nested_fixture() {
        let (root, warnings) = normalize_call_frame(&fixture());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(root.kind, CallKind::Call);
        assert_eq!(root.id, 0);
        assert_eq!(root.children.len(), 3);
        assert_eq!(root.count(), 5);

        // Child 0: token call emitting a Transfer, with one nested STATICCALL.
        let token_call = &root.children[0];
        assert_eq!(token_call.kind, CallKind::Call);
        assert_eq!(token_call.logs.len(), 1);
        assert_eq!(token_call.logs[0].position, Some(1));
        let dec = token_call.logs[0].decoded.as_ref().unwrap();
        assert_eq!(dec.name, "Transfer");
        assert_eq!(token_call.decoded.as_ref().unwrap().name.as_deref(), Some("transfer"));

        // Child 1: reverted subcall with reason decoded from output.
        let reverted = &root.children[1];
        assert_eq!(reverted.error.as_deref(), Some("execution reverted"));
        assert_eq!(reverted.revert_reason.as_deref(), Some("nope"));

        // Child 2: plain value transfer.
        let send = &root.children[2];
        assert_eq!(send.value, U256::from(1u8));
        assert_eq!(send.gas_used, 0);
    }

    #[test]
    fn assigns_log_indices_in_execution_order() {
        let (mut root, _) = normalize_call_frame(&fixture());
        let receipt_logs = vec![ReceiptLog {
            address: address!("0x00000000000000000000000000000000000a11ce"),
            topics: vec![
                b256!("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"),
                b256!("0x0000000000000000000000002000000000000000000000000000000000000002"),
                b256!("0x0000000000000000000000003000000000000000000000000000000000000003"),
            ],
            data: Bytes::from(U256::from(5u8).to_be_bytes::<32>().to_vec()),
            log_index: Some(42),
        }];
        assert!(assign_log_indices(&mut root, &receipt_logs));
        assert_eq!(root.children[0].logs[0].log_index, Some(42));

        // Mismatched receipt → refuse to assign.
        let wrong = vec![ReceiptLog {
            address: address!("0x00000000000000000000000000000000000000ff"),
            topics: vec![],
            data: Bytes::new(),
            log_index: Some(0),
        }];
        assert!(!assign_log_indices(&mut root, &wrong));
    }
}
