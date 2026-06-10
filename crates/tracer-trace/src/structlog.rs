//! OpenTracer-style struct-log interpretation: rebuild the call tree, storage
//! accesses, and event attribution from a raw `debug_traceTransaction`
//! opcode stream — no proprietary tracer required.
//!
//! Handles the delicate parts of the encoding:
//! - calls to code-less accounts (EOAs, precompiles) do not bump `depth` and
//!   complete in place, with the success flag on the caller's resume stack;
//! - `DELEGATECALL`/`CALLCODE` children execute in the caller's context, so
//!   their logs and storage writes attribute to the caller's address;
//! - `CREATE` addresses only become visible on the parent's resume stack
//!   (constructor logs are patched at frame close); `CREATE2` addresses are
//!   precomputed from the salt and init code;
//! - revert reasons decode from the `REVERT` operand memory range.

use alloy_primitives::{Address, B256, Bytes, U256, hex};
use alloy_rpc_types_trace::geth::{DefaultFrame, StructLog};
use std::collections::HashMap;
use tracer_core::{CallKind, Frame, FrameLog, StorageRead, StorageWrite, decode};

/// Top-level call context (struct logs do not carry it themselves).
#[derive(Clone, Debug)]
pub struct RootCall {
    pub from: Address,
    /// Callee. For creation transactions pass the created contract address
    /// from the receipt (if known) so constructor activity attributes
    /// correctly.
    pub to: Option<Address>,
    /// Whether this is a contract-creation transaction.
    pub create: bool,
    pub value: U256,
    /// Calldata, or init code for creations.
    pub input: Bytes,
    pub gas_limit: u64,
}

#[derive(Clone, Debug)]
pub struct InterpretOutcome {
    pub root: Frame,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum StructLogError {
    #[error("malformed struct-log stream: {0}")]
    Malformed(String),
}

struct Open {
    frame: Frame,
    /// `ADDRESS` opcode value for this frame (log/storage attribution).
    context: Address,
    entry_gas: u64,
}

/// Safety cap for a single memory read (inputs, return data, log data).
const MAX_MEM_READ: usize = 1 << 22;

/// Interpret a default-tracer struct-log stream into a normalized [`Frame`]
/// tree. Degrades gracefully (with warnings) when stack/memory capture was
/// disabled in the tracer config.
pub fn interpret_struct_logs(
    df: &DefaultFrame,
    root: &RootCall,
) -> Result<InterpretOutcome, StructLogError> {
    let mut warnings: Vec<String> = Vec::new();
    let mut mem_warned = false;

    let root_kind = if root.create { CallKind::Create } else { CallKind::Call };
    let mut rf = Frame::new(root_kind, root.from);
    rf.to = root.to;
    rf.value = root.value;
    rf.input = root.input.clone();
    rf.gas = root.gas_limit;
    if !root.create {
        rf.decoded = decode::decode_call(root.input.as_ref());
    }
    let root_ctx = root.to.unwrap_or(Address::ZERO);

    let mut stack: Vec<Open> =
        vec![Open { frame: rf, context: root_ctx, entry_gas: root.gas_limit }];
    let mut storage_seen: HashMap<(Address, B256), B256> = HashMap::new();

    let logs = &df.struct_logs;
    for i in 0..logs.len() {
        let sl = &logs[i];
        let depth = sl.depth as usize;
        if depth == 0 {
            return Err(StructLogError::Malformed(format!("zero depth at step {i}")));
        }

        while stack.len() > depth {
            let prev = &logs[i - 1];
            close_frame(&mut stack, prev, Some(sl), &mut warnings, &mut mem_warned)?;
        }
        if stack.len() < depth {
            return Err(StructLogError::Malformed(format!(
                "depth jumped from {} to {depth} at step {i}",
                stack.len()
            )));
        }

        match &*sl.op {
            op @ ("CALL" | "CALLCODE" | "DELEGATECALL" | "STATICCALL") => {
                let has_value = matches!(op, "CALL" | "CALLCODE");
                let kind = match op {
                    "CALL" => CallKind::Call,
                    "CALLCODE" => CallKind::CallCode,
                    "DELEGATECALL" => CallKind::DelegateCall,
                    _ => CallKind::StaticCall,
                };
                let gas_req = stack_top(sl, 0).map(u256_to_u64).unwrap_or_default();
                let to = stack_top(sl, 1).map(u256_to_addr);
                let value = if has_value { stack_top(sl, 2) } else { None };
                let base = if has_value { 3 } else { 2 };
                let in_off = stack_top(sl, base);
                let in_len = stack_top(sl, base + 1);
                let out_off = stack_top(sl, base + 2);
                let out_len = stack_top(sl, base + 3);
                let input = read_mem(sl, in_off, in_len, &mut mem_warned, &mut warnings);

                let cur = stack.last_mut().expect("frame stack never empty");
                let mut child = Frame::new(kind, cur.context);
                child.to = to;
                child.value = if has_value { value.unwrap_or_default() } else { U256::ZERO };
                child.gas = gas_req;
                child.input = input;
                child.decoded = decode::decode_call(child.input.as_ref());

                if next_depth(logs, i) == Some(depth + 1) {
                    let child_ctx = match kind {
                        CallKind::DelegateCall | CallKind::CallCode => cur.context,
                        _ => to.unwrap_or(Address::ZERO),
                    };
                    let entry_gas = logs[i + 1].gas;
                    stack.push(Open { frame: child, context: child_ctx, entry_gas });
                } else {
                    // Code-less callee (EOA / precompile): completed in place;
                    // success flag and any output are visible on the next row.
                    if let Some(next) = logs.get(i + 1) {
                        let ok = stack_top(next, 0).map(|v| !v.is_zero()).unwrap_or(true);
                        if !ok {
                            child.error = Some("call failed".into());
                        } else if let (Some(off), Some(len)) = (out_off, out_len)
                            && !len.is_zero()
                        {
                            child.output = read_mem(
                                next,
                                Some(off),
                                Some(len),
                                &mut mem_warned,
                                &mut warnings,
                            );
                        }
                    }
                    stack.last_mut().expect("frame stack never empty").frame.children.push(child);
                }
            }
            op @ ("CREATE" | "CREATE2") => {
                let kind = if op == "CREATE" { CallKind::Create } else { CallKind::Create2 };
                let value = stack_top(sl, 0).unwrap_or_default();
                let off = stack_top(sl, 1);
                let len = stack_top(sl, 2);
                let initcode = read_mem(sl, off, len, &mut mem_warned, &mut warnings);
                let cur_ctx = stack.last().expect("frame stack never empty").context;
                let mut child = Frame::new(kind, cur_ctx);
                child.value = value;
                child.input = initcode;
                // CREATE2 addresses are a pure function of (deployer, salt,
                // init code); plain CREATE needs the deployer nonce, so the
                // address is patched at close from the parent's resume stack.
                if kind == CallKind::Create2
                    && let Some(salt) = stack_top(sl, 3)
                {
                    child.to =
                        Some(cur_ctx.create2_from_code(B256::from(salt), child.input.as_ref()));
                }
                if next_depth(logs, i) == Some(depth + 1) {
                    let ctx = child.to.unwrap_or(Address::ZERO);
                    let entry_gas = logs[i + 1].gas;
                    stack.push(Open { frame: child, context: ctx, entry_gas });
                } else {
                    if let Some(next) = logs.get(i + 1) {
                        let created =
                            stack_top(next, 0).map(u256_to_addr).filter(|a| *a != Address::ZERO);
                        if created.is_some() {
                            child.to = created;
                        } else if child.error.is_none() {
                            child.error = Some("create failed".into());
                        }
                    }
                    stack.last_mut().expect("frame stack never empty").frame.children.push(child);
                }
            }
            op @ ("LOG0" | "LOG1" | "LOG2" | "LOG3" | "LOG4") => {
                let n = (op.as_bytes()[3] - b'0') as usize;
                let off = stack_top(sl, 0);
                let len = stack_top(sl, 1);
                let mut topics = Vec::with_capacity(n);
                for j in 0..n {
                    if let Some(t) = stack_top(sl, 2 + j) {
                        topics.push(B256::from(t));
                    }
                }
                let data = read_mem(sl, off, len, &mut mem_warned, &mut warnings);
                let cur = stack.last_mut().expect("frame stack never empty");
                let address = cur.context;
                let decoded = decode::decode_event(address, &topics, data.as_ref());
                let position = Some(cur.frame.children.len() as u64);
                cur.frame.logs.push(FrameLog {
                    address,
                    topics,
                    data,
                    position,
                    log_index: None,
                    decoded,
                });
            }
            "SLOAD" => {
                if let Some(slot_u) = stack_top(sl, 0) {
                    let slot = B256::from(slot_u);
                    // Value from the storage map when captured, else from the
                    // pushed result on the next row.
                    let value =
                        sl.storage.as_ref().and_then(|m| m.get(&slot).copied()).or_else(|| {
                            logs.get(i + 1)
                                .filter(|n| n.depth == sl.depth && n.error.is_none())
                                .and_then(|n| stack_top(n, 0))
                                .map(B256::from)
                        });
                    let cur = stack.last_mut().expect("frame stack never empty");
                    if let Some(v) = value {
                        storage_seen.insert((cur.context, slot), v);
                    }
                    cur.frame.storage_reads.push(StorageRead { slot, value });
                }
            }
            "SSTORE" => {
                if let (Some(slot_u), Some(val)) = (stack_top(sl, 0), stack_top(sl, 1)) {
                    let slot = B256::from(slot_u);
                    let value = B256::from(val);
                    let cur = stack.last_mut().expect("frame stack never empty");
                    let previous = storage_seen.insert((cur.context, slot), value);
                    cur.frame.storage_writes.push(StorageWrite { slot, previous, value });
                }
            }
            "SELFDESTRUCT" => {
                let beneficiary = stack_top(sl, 0).map(u256_to_addr);
                let cur = stack.last_mut().expect("frame stack never empty");
                let mut sd = Frame::new(CallKind::SelfDestruct, cur.context);
                sd.to = beneficiary;
                cur.frame.children.push(sd);
                push_once(
                    &mut warnings,
                    "SELFDESTRUCT amount is not recoverable from struct logs; \
                     merged callTracer data carries the exact value",
                );
            }
            _ => {}
        }
    }

    // Unwind anything still open (halt at depth, or a truncated stream).
    while stack.len() > 1 {
        let prev = logs
            .last()
            .ok_or_else(|| StructLogError::Malformed("empty stream with open frames".into()))?;
        close_frame(&mut stack, prev, None, &mut warnings, &mut mem_warned)?;
    }

    let mut root_frame = stack.pop().expect("root frame").frame;
    root_frame.output = df.return_value.clone();
    root_frame.gas_used = df.gas;
    if df.failed && root_frame.error.is_none() {
        root_frame.error = Some(match logs.last() {
            Some(last) => last.error.clone().unwrap_or_else(|| {
                if &*last.op == "REVERT" {
                    "execution reverted".into()
                } else {
                    "execution failed".into()
                }
            }),
            None => "execution failed".into(),
        });
    }
    if df.failed && root_frame.revert_reason.is_none() {
        root_frame.revert_reason = decode::revert_reason(root_frame.output.as_ref());
    }
    root_frame.assign_ids();
    Ok(InterpretOutcome { root: root_frame, warnings })
}

/// Close the deepest open frame. `prev` is the last row executed inside it;
/// `resume` is the caller's first row afterwards (absent at stream end).
fn close_frame(
    stack: &mut Vec<Open>,
    prev: &StructLog,
    resume: Option<&StructLog>,
    warnings: &mut Vec<String>,
    mem_warned: &mut bool,
) -> Result<(), StructLogError> {
    let Open { mut frame, context, entry_gas } =
        stack.pop().ok_or_else(|| StructLogError::Malformed("close on empty stack".into()))?;

    match &*prev.op {
        op @ ("RETURN" | "REVERT") => {
            let off = stack_top(prev, 0);
            let len = stack_top(prev, 1);
            let data = read_mem(prev, off, len, mem_warned, warnings);
            if op == "REVERT" {
                frame.error = Some("execution reverted".into());
                frame.revert_reason = decode::revert_reason(data.as_ref());
            }
            frame.output = data;
        }
        "STOP" | "SELFDESTRUCT" => {}
        "INVALID" => frame.error = Some("invalid opcode".into()),
        _ => {}
    }
    if let Some(e) = &prev.error
        && frame.error.is_none()
    {
        frame.error = Some(e.clone());
    }
    frame.gas_used = entry_gas.saturating_sub(prev.gas).saturating_add(prev.gas_cost);

    if let Some(res) = resume {
        if frame.kind.is_create() {
            let created = stack_top(res, 0).map(u256_to_addr).filter(|a| *a != Address::ZERO);
            match created {
                Some(addr) => {
                    frame.to = Some(addr);
                    // Constructor logs recorded before the address was known.
                    if context == Address::ZERO {
                        for l in &mut frame.logs {
                            if l.address == Address::ZERO {
                                l.address = addr;
                                if l.decoded.is_none() {
                                    l.decoded =
                                        decode::decode_event(addr, &l.topics, l.data.as_ref());
                                }
                            }
                        }
                    }
                }
                None => {
                    if frame.error.is_none() {
                        frame.error = Some("create failed".into());
                    }
                    frame.to = None;
                }
            }
        } else if frame.error.is_none()
            && let Some(ok) = stack_top(res, 0)
            && ok.is_zero()
        {
            frame.error = Some("call failed".into());
        }
    }

    let parent =
        stack.last_mut().ok_or_else(|| StructLogError::Malformed("orphan frame".into()))?;
    parent.frame.children.push(frame);
    Ok(())
}

fn next_depth(logs: &[StructLog], i: usize) -> Option<usize> {
    logs.get(i + 1).map(|n| n.depth as usize)
}

/// `n`-th stack operand from the top (geth lists the stack bottom-first).
fn stack_top(sl: &StructLog, n: usize) -> Option<U256> {
    let st = sl.stack.as_ref()?;
    st.len().checked_sub(1 + n).and_then(|idx| st.get(idx)).copied()
}

fn u256_to_addr(v: U256) -> Address {
    Address::from_word(B256::from(v))
}

fn u256_to_u64(v: U256) -> u64 {
    u64::try_from(v).unwrap_or(u64::MAX)
}

/// Read `[off, off+len)` from a struct-log memory snapshot (32-byte hex
/// words). Out-of-range bytes read as zero, matching EVM semantics.
fn read_mem(
    sl: &StructLog,
    off: Option<U256>,
    len: Option<U256>,
    mem_warned: &mut bool,
    warnings: &mut Vec<String>,
) -> Bytes {
    let (Some(off), Some(len)) = (off, len) else { return Bytes::new() };
    if len.is_zero() {
        return Bytes::new();
    }
    let (Ok(off), Ok(len)) = (usize::try_from(off), usize::try_from(len)) else {
        return Bytes::new();
    };
    if len > MAX_MEM_READ {
        return Bytes::new();
    }
    let Some(mem) = sl.memory.as_ref() else {
        if !*mem_warned {
            warnings.push(
                "struct logs carry no memory snapshots; call inputs and event data are \
                 degraded (run the tracer with memory capture enabled)"
                    .into(),
            );
            *mem_warned = true;
        }
        return Bytes::new();
    };
    let mut out = vec![0u8; len];
    let w_start = off / 32;
    let w_end = (off + len).div_ceil(32);
    for w in w_start..w_end {
        let Some(word_hex) = mem.get(w) else { break };
        let word_hex = word_hex.strip_prefix("0x").unwrap_or(word_hex);
        let Ok(word) = hex::decode(word_hex) else { continue };
        for (bi, byte) in word.iter().enumerate() {
            let abs = w * 32 + bi;
            if abs >= off && abs < off + len {
                out[abs - off] = *byte;
            }
        }
    }
    Bytes::from(out)
}

fn push_once(warnings: &mut Vec<String>, msg: &str) {
    if !warnings.iter().any(|w| w == msg) {
        warnings.push(msg.into());
    }
}
