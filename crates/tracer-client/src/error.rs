use alloy_primitives::B256;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("invalid RPC URL: {0}")]
    InvalidUrl(String),
    #[error("transaction {0} not found on this endpoint")]
    TxNotFound(B256),
    #[error("transaction {0} is not mined yet")]
    NotMined(B256),
    #[error("block {0} not found")]
    BlockNotFound(u64),
    #[error("RPC error: {0}")]
    Rpc(String),
    #[error(
        "endpoint does not support debug_traceTransaction ({0}); \
         retry with `--backend anvil-fork` or use a debug-enabled node"
    )]
    DebugUnsupported(String),
    #[error("anvil fork failed: {0}. Is `anvil` (Foundry) installed and on PATH?")]
    Anvil(String),
    #[error("replaying the transaction on the fork failed: {0}")]
    Replay(String),
}

impl ClientError {
    pub(crate) fn rpc(e: impl std::fmt::Display) -> Self {
        Self::Rpc(e.to_string())
    }
}
