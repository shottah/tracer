//! Asset transfers — the shared substrate from which balance changes and the
//! fund-flow graph are derived.

use crate::amount::Amount;
use alloy_primitives::{Address, B256, Bytes, U256};
use serde::{Deserialize, Serialize};

/// The asset moved by a transfer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Asset {
    /// The chain's native currency (ETH on mainnet).
    Native,
    Erc20 {
        token: Address,
    },
    Erc721 {
        token: Address,
        #[serde(rename = "tokenId")]
        token_id: U256,
    },
    Erc1155 {
        token: Address,
        #[serde(rename = "tokenId")]
        token_id: U256,
    },
}

impl Asset {
    pub fn token(&self) -> Option<Address> {
        match self {
            Self::Native => None,
            Self::Erc20 { token } | Self::Erc721 { token, .. } | Self::Erc1155 { token, .. } => {
                Some(*token)
            }
        }
    }

    /// Grouping key: one balance-change row per distinct key per account.
    pub fn key(&self) -> String {
        match self {
            Self::Native => "native".into(),
            Self::Erc20 { token } => format!("erc20:{token}"),
            Self::Erc721 { token, token_id } => format!("erc721:{token}:{token_id}"),
            Self::Erc1155 { token, token_id } => format!("erc1155:{token}:{token_id}"),
        }
    }
}

/// Where a transfer was observed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TransferOrigin {
    /// The transaction's own `value`.
    TxValue,
    /// A `CALL` with value (frame id in the trace).
    Call { frame_id: u32 },
    /// A `CREATE`/`CREATE2` endowment.
    Create { frame_id: u32 },
    /// A `SELFDESTRUCT` balance sweep.
    SelfDestruct { frame_id: u32 },
    /// A token transfer event.
    Log {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        log_index: Option<u64>,
    },
    /// Wrapped-native mint (`Deposit`).
    Deposit {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        log_index: Option<u64>,
    },
    /// Wrapped-native burn (`Withdrawal`).
    Withdrawal {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        log_index: Option<u64>,
    },
}

/// One movement of one asset between two addresses, in execution order.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetTransfer {
    /// Position in the transaction's execution order (0-based).
    pub order: u32,
    pub from: Address,
    pub to: Address,
    pub asset: Asset,
    pub amount: Amount,
    pub origin: TransferOrigin,
}

/// A receipt log, decoupled from any RPC client type.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptLog {
    pub address: Address,
    pub topics: Vec<B256>,
    pub data: Bytes,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub log_index: Option<u64>,
}
