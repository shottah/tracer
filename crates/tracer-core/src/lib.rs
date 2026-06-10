//! Core data model for the `tracer` EVM transaction inspector.
//!
//! This crate is pure: it depends only on `alloy-primitives`/`alloy-sol-types`
//! and never touches the network. All public types serialize to camelCase JSON
//! designed to be rendered directly by a UI (see `docs/formats.md`).

pub mod amount;
pub mod balance;
pub mod decode;
pub mod fundflow;
pub mod labels;
pub mod report;
pub mod trace;
pub mod transfer;

pub use amount::{Amount, SignedAmount, format_units};
pub use balance::{AccountBalanceChange, BalanceChanges, NativeChange, NativeSource, TokenChange};
pub use fundflow::{FlowEdge, FlowNode, FundFlow, NodeKind};
pub use report::{
    BackendInfo, BackendKind, FidelityCheck, ForkInfo, SCHEMA_VERSION, TokenInfo, TokenStandard,
    TraceReport, TxSummary,
};
pub use trace::{
    CallKind, DecodedCall, DecodedEvent, DecodedParam, Frame, FrameLog, StorageRead, StorageWrite,
};
pub use transfer::{Asset, AssetTransfer, ReceiptLog, TransferOrigin};
