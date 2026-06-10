//! Pure analyzers over normalized traces: asset transfers, balance changes,
//! and the fund-flow graph.

pub mod balance;
pub mod fundflow;
pub mod transfers;

pub use balance::{BalanceInput, compute_balance_changes};
pub use fundflow::build_fund_flow;
pub use transfers::{TransferContext, collect_transfers};
