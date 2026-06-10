//! Merge deep (struct-log) data into a `callTracer`-normalized tree.
//!
//! The fast tree is authoritative for amounts, gas, and errors; the deep tree
//! contributes storage accesses and (when the fast path lacked `withLog`
//! support) event data. Trees must align structurally — same call shape — or
//! the merge is refused.

use tracer_core::Frame;

/// Returns `false` (leaving `fast` untouched) when the trees do not align.
pub fn merge_deep_into(fast: &mut Frame, deep: &Frame) -> bool {
    if !aligned(fast, deep) {
        return false;
    }
    apply(fast, deep);
    true
}

fn aligned(a: &Frame, b: &Frame) -> bool {
    a.kind == b.kind
        && a.children.len() == b.children.len()
        && a.children.iter().zip(&b.children).all(|(x, y)| aligned(x, y))
}

fn apply(fast: &mut Frame, deep: &Frame) {
    fast.storage_reads = deep.storage_reads.clone();
    fast.storage_writes = deep.storage_writes.clone();
    if fast.logs.is_empty() && !deep.logs.is_empty() {
        fast.logs = deep.logs.clone();
    }
    for (f, d) in fast.children.iter_mut().zip(&deep.children) {
        apply(f, d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, B256};
    use tracer_core::{CallKind, StorageWrite};

    #[test]
    fn merges_storage_and_refuses_misaligned() {
        let a = Address::ZERO;
        let mut fast = Frame::new(CallKind::Call, a);
        fast.children.push(Frame::new(CallKind::StaticCall, a));
        let mut deep = fast.clone();
        deep.children[0].storage_writes.push(StorageWrite {
            slot: B256::ZERO,
            previous: None,
            value: B256::with_last_byte(1),
        });

        assert!(merge_deep_into(&mut fast, &deep));
        assert_eq!(fast.children[0].storage_writes.len(), 1);

        let mut other = Frame::new(CallKind::Call, a);
        other.children.push(Frame::new(CallKind::DelegateCall, a));
        assert!(!merge_deep_into(&mut fast, &other));
    }
}
