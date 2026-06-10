//! Normalized call-trace model: a tree of [`Frame`]s with logs, storage
//! accesses, and decoded call/event annotations.

use alloy_primitives::{Address, B256, Bytes, U256};
use serde::{Deserialize, Serialize};

/// The kind of a call frame, matching geth `callTracer` type strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallKind {
    #[serde(rename = "CALL")]
    Call,
    #[serde(rename = "STATICCALL")]
    StaticCall,
    #[serde(rename = "DELEGATECALL")]
    DelegateCall,
    #[serde(rename = "CALLCODE")]
    CallCode,
    #[serde(rename = "CREATE")]
    Create,
    #[serde(rename = "CREATE2")]
    Create2,
    #[serde(rename = "SELFDESTRUCT")]
    SelfDestruct,
}

impl CallKind {
    pub fn from_geth(s: &str) -> Option<Self> {
        Some(match s.to_ascii_uppercase().as_str() {
            "CALL" => Self::Call,
            "STATICCALL" => Self::StaticCall,
            "DELEGATECALL" => Self::DelegateCall,
            "CALLCODE" => Self::CallCode,
            "CREATE" => Self::Create,
            "CREATE2" => Self::Create2,
            "SELFDESTRUCT" | "SUICIDE" => Self::SelfDestruct,
            _ => return None,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Call => "CALL",
            Self::StaticCall => "STATICCALL",
            Self::DelegateCall => "DELEGATECALL",
            Self::CallCode => "CALLCODE",
            Self::Create => "CREATE",
            Self::Create2 => "CREATE2",
            Self::SelfDestruct => "SELFDESTRUCT",
        }
    }

    pub fn is_create(&self) -> bool {
        matches!(self, Self::Create | Self::Create2)
    }

    /// Whether a non-zero `value` on this frame moves native currency between
    /// distinct accounts. DELEGATECALL/STATICCALL never carry value;
    /// CALLCODE "transfers" to self (no net flow).
    pub fn moves_value(&self) -> bool {
        matches!(self, Self::Call | Self::Create | Self::Create2 | Self::SelfDestruct)
    }
}

/// One node of the call tree.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    /// Stable DFS pre-order index within the trace.
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parent: Option<u32>,
    /// 0 for the root frame.
    pub depth: u32,
    pub kind: CallKind,
    pub from: Address,
    /// Callee; for CREATE the created address; `None` for failed creates.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub to: Option<Address>,
    pub value: U256,
    pub gas: u64,
    pub gas_used: u64,
    pub input: Bytes,
    pub output: Bytes,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub revert_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub decoded: Option<DecodedCall>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub logs: Vec<FrameLog>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub storage_reads: Vec<StorageRead>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub storage_writes: Vec<StorageWrite>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<Frame>,
}

impl Frame {
    /// A blank frame; callers fill in what they know and finish with
    /// [`Frame::assign_ids`] on the root.
    pub fn new(kind: CallKind, from: Address) -> Self {
        Self {
            id: 0,
            parent: None,
            depth: 0,
            kind,
            from,
            to: None,
            value: U256::ZERO,
            gas: 0,
            gas_used: 0,
            input: Bytes::new(),
            output: Bytes::new(),
            error: None,
            revert_reason: None,
            decoded: None,
            logs: Vec::new(),
            storage_reads: Vec::new(),
            storage_writes: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Frame executed without an error of its own.
    pub fn ok(&self) -> bool {
        self.error.is_none()
    }

    /// Assign DFS pre-order `id`, `parent`, and `depth` over the whole tree.
    pub fn assign_ids(&mut self) {
        fn rec(f: &mut Frame, next: &mut u32, parent: Option<u32>, depth: u32) {
            f.id = *next;
            *next += 1;
            f.parent = parent;
            f.depth = depth;
            let id = f.id;
            for c in &mut f.children {
                rec(c, next, Some(id), depth + 1);
            }
        }
        let mut next = 0;
        rec(self, &mut next, None, 0);
    }

    /// DFS pre-order iterator over the tree.
    pub fn iter(&self) -> FrameIter<'_> {
        FrameIter { stack: vec![self] }
    }

    pub fn count(&self) -> usize {
        self.iter().count()
    }
}

pub struct FrameIter<'a> {
    stack: Vec<&'a Frame>,
}

impl<'a> Iterator for FrameIter<'a> {
    type Item = &'a Frame;

    fn next(&mut self) -> Option<Self::Item> {
        let f = self.stack.pop()?;
        self.stack.extend(f.children.iter().rev());
        Some(f)
    }
}

/// An event emitted within a frame.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameLog {
    /// Emitting context address (the `ADDRESS` opcode value — for
    /// DELEGATECALL frames this is the caller's context, as on chain).
    pub address: Address,
    pub topics: Vec<B256>,
    pub data: Bytes,
    /// Number of sibling subcalls executed before this log (geth `withLog`
    /// semantics); lets consumers interleave calls and events exactly.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub position: Option<u64>,
    /// Receipt log index, when this log could be matched to the receipt.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub log_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub decoded: Option<DecodedEvent>,
}

/// An `SLOAD` observed in deep (struct-log) mode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRead {
    pub slot: B256,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<B256>,
}

/// An `SSTORE` observed in deep (struct-log) mode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageWrite {
    pub slot: B256,
    /// Last value seen for this slot earlier in the transaction, if any.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub previous: Option<B256>,
    pub value: B256,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedParam {
    pub name: String,
    pub value: String,
}

/// Best-effort decoding of a call's input data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedCall {
    /// 4-byte selector as `0x`-hex (always present when input >= 4 bytes).
    pub selector: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub params: Vec<DecodedParam>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedEvent {
    pub name: String,
    pub signature: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub params: Vec<DecodedParam>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    #[test]
    fn ids_and_iteration() {
        let a = address!("0x1111111111111111111111111111111111111111");
        let mut root = Frame::new(CallKind::Call, a);
        let mut c0 = Frame::new(CallKind::StaticCall, a);
        c0.children.push(Frame::new(CallKind::DelegateCall, a));
        root.children.push(c0);
        root.children.push(Frame::new(CallKind::Create, a));
        root.assign_ids();

        let ids: Vec<u32> = root.iter().map(|f| f.id).collect();
        assert_eq!(ids, vec![0, 1, 2, 3]);
        assert_eq!(root.children[0].children[0].parent, Some(1));
        assert_eq!(root.children[0].children[0].depth, 2);
        assert_eq!(root.children[1].parent, Some(0));
        assert_eq!(root.count(), 4);
    }

    #[test]
    fn callkind_serde_matches_geth() {
        assert_eq!(serde_json::to_string(&CallKind::StaticCall).unwrap(), "\"STATICCALL\"");
        assert_eq!(CallKind::from_geth("delegatecall"), Some(CallKind::DelegateCall));
        assert_eq!(CallKind::from_geth("SUICIDE"), Some(CallKind::SelfDestruct));
        assert_eq!(CallKind::from_geth("nope"), None);
    }
}
