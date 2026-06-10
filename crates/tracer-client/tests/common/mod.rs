//! Shared e2e harness: hand-assembled EVM contracts and transaction helpers
//! for hermetic tests against a local (offline) anvil node.
//!
//! Tests skip gracefully when `anvil` is not installed; CI installs Foundry
//! so they always run there.

#![allow(dead_code)]

use alloy::{
    network::TransactionBuilder,
    node_bindings::{Anvil, AnvilInstance},
    providers::{DynProvider, Provider, ProviderBuilder},
    rpc::types::{TransactionReceipt, TransactionRequest},
};
use alloy_primitives::{Address, B256, Bytes, TxKind, U256};

/// keccak256("Transfer(address,address,uint256)")
pub const TRANSFER_SIG: [u8; 32] = [
    0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b, 0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37, 0x8d, 0xaa,
    0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16, 0x28, 0xf5, 0x5a, 0x4d, 0xf5, 0x23, 0xb3, 0xef,
];

/// Spawn anvil with extra args, or skip the test when unavailable.
pub fn spawn_anvil(args: &[&str]) -> Option<AnvilInstance> {
    let mut anvil = Anvil::new();
    if !args.is_empty() {
        anvil = anvil.args(args.iter().map(|s| s.to_string()));
    }
    match anvil.try_spawn() {
        Ok(instance) => Some(instance),
        Err(e) => {
            eprintln!("SKIPPED: anvil unavailable ({e}); install Foundry to run e2e tests");
            None
        }
    }
}

pub fn provider_for(anvil: &AnvilInstance) -> DynProvider {
    ProviderBuilder::new().connect_http(anvil.endpoint_url()).erased()
}

// ---------------------------------------------------------------------------
// Tiny EVM assembler — just enough opcodes for the test contracts.
// ---------------------------------------------------------------------------

pub mod asm {
    pub const STOP: u8 = 0x00;
    pub const CALLDATALOAD: u8 = 0x35;
    pub const CODECOPY: u8 = 0x39;
    pub const CALLER: u8 = 0x33;
    pub const POP: u8 = 0x50;
    pub const MSTORE: u8 = 0x52;
    pub const SSTORE: u8 = 0x55;
    pub const GAS: u8 = 0x5a;
    pub const PUSH0: u8 = 0x5f;
    pub const LOG3: u8 = 0xa3;
    pub const CALL: u8 = 0xf1;
    pub const RETURN: u8 = 0xf3;
    pub const REVERT: u8 = 0xfd;

    /// `PUSH<n>` for 1..=32 bytes of immediate data.
    pub fn push(code: &mut Vec<u8>, data: &[u8]) {
        assert!(!data.is_empty() && data.len() <= 32, "push takes 1..=32 bytes");
        code.push(0x60 + (data.len() as u8 - 1));
        code.extend_from_slice(data);
    }

    pub fn push_byte(code: &mut Vec<u8>, b: u8) {
        push(code, &[b]);
    }

    /// Standard deploy wrapper: copy the runtime to memory and return it.
    ///
    /// Layout (12 bytes):
    /// `PUSH2 len, PUSH1 12, PUSH0, CODECOPY, PUSH2 len, PUSH0, RETURN`
    pub fn initcode(runtime: &[u8]) -> Vec<u8> {
        let len = (runtime.len() as u16).to_be_bytes();
        let mut c = Vec::with_capacity(12 + runtime.len());
        // CODECOPY(destOffset=0, offset=12, length=len): push length, offset, dest.
        c.push(0x61); // PUSH2
        c.extend_from_slice(&len);
        c.push(0x60); // PUSH1
        c.push(12);
        c.push(PUSH0);
        c.push(CODECOPY);
        // RETURN(offset=0, size=len): push size, then offset.
        c.push(0x61); // PUSH2
        c.extend_from_slice(&len);
        c.push(PUSH0);
        c.push(RETURN);
        c.extend_from_slice(runtime);
        c
    }
}

/// Token-ish contract: on any call, emits
/// `Transfer(caller, calldata[4..36], calldata[36..68])` and returns true.
pub fn token_runtime() -> Vec<u8> {
    use asm::*;
    let mut c = Vec::new();
    // mem[0..32] = amount (MSTORE pops offset from the top: push value, then offset)
    push_byte(&mut c, 0x24);
    c.push(CALLDATALOAD);
    c.push(PUSH0);
    c.push(MSTORE);
    // LOG3(offset=0, size=32, sig, caller, to): push t3, t2, t1, size, offset.
    push_byte(&mut c, 0x04);
    c.push(CALLDATALOAD); // to (topic3)
    c.push(CALLER); // topic2
    push(&mut c, &TRANSFER_SIG); // topic1
    push_byte(&mut c, 0x20); // size
    c.push(PUSH0); // offset
    c.push(LOG3);
    // return true
    push_byte(&mut c, 0x01);
    c.push(PUSH0);
    c.push(MSTORE);
    push_byte(&mut c, 0x20);
    c.push(PUSH0);
    c.push(RETURN);
    c
}

/// Always reverts with empty data.
pub fn reverter_runtime() -> Vec<u8> {
    use asm::*;
    vec![PUSH0, PUSH0, REVERT]
}

/// Orchestrator:
/// 1. sends 1 wei to `eoa1`
/// 2. calls `token.transfer(eoa2, 5)` (emits Transfer)
/// 3. calls `reverter` (low-level, ignores failure)
/// 4. `SSTORE(7, 42)`
pub fn main_runtime(eoa1: Address, token: Address, eoa2: Address, reverter: Address) -> Vec<u8> {
    use asm::*;
    let mut c = Vec::new();

    // 1) CALL(gas, eoa1, 1, 0, 0, 0, 0): push retLen, retOff, argsLen, argsOff, value, addr, gas.
    c.extend([PUSH0; 4]);
    push_byte(&mut c, 0x01);
    push(&mut c, eoa1.as_slice());
    c.push(GAS);
    c.push(CALL);
    c.push(POP);

    // 2) calldata: selector | eoa2 | 5
    let mut selector_word = [0u8; 32];
    selector_word[..4].copy_from_slice(&[0xa9, 0x05, 0x9c, 0xbb]);
    push(&mut c, &selector_word);
    c.push(PUSH0);
    c.push(MSTORE);
    push(&mut c, eoa2.as_slice()); // right-aligned 20 bytes
    push_byte(&mut c, 0x04);
    c.push(MSTORE);
    push_byte(&mut c, 0x05);
    push_byte(&mut c, 0x24);
    c.push(MSTORE);
    // CALL(gas, token, 0, 0, 68, 0, 32)
    push_byte(&mut c, 0x20);
    c.push(PUSH0);
    push_byte(&mut c, 0x44);
    c.push(PUSH0);
    c.push(PUSH0);
    push(&mut c, token.as_slice());
    c.push(GAS);
    c.push(CALL);
    c.push(POP);

    // 3) CALL(gas, reverter, 0, 0, 0, 0, 0)
    c.extend([PUSH0; 5]);
    push(&mut c, reverter.as_slice());
    c.push(GAS);
    c.push(CALL);
    c.push(POP);

    // 4) SSTORE(7, 42): pops key from the top — push value, then key.
    push_byte(&mut c, 0x2a);
    push_byte(&mut c, 0x07);
    c.push(SSTORE);
    c.push(STOP);
    c
}

// ---------------------------------------------------------------------------
// Transaction helpers (explicit nonce/gas so queued txs work under
// --no-mining without filler interference).
// ---------------------------------------------------------------------------

pub struct TxSender {
    pub provider: DynProvider,
    pub from: Address,
    pub nonce: u64,
    pub gas_price: u128,
}

impl TxSender {
    pub async fn new(provider: DynProvider, from: Address) -> Self {
        let nonce = provider.get_transaction_count(from).await.expect("nonce");
        let gas_price = provider.get_gas_price().await.expect("gas price");
        Self { provider, from, nonce, gas_price }
    }

    pub async fn send(&mut self, to: Option<Address>, value: u64, data: Vec<u8>, gas: u64) -> B256 {
        let mut req = TransactionRequest::default()
            .with_from(self.from)
            .with_value(U256::from(value))
            .with_nonce(self.nonce)
            .with_gas_limit(gas)
            .with_gas_price(self.gas_price)
            .with_input(Bytes::from(data));
        req = match to {
            Some(addr) => req.with_to(addr),
            None => req.with_kind(TxKind::Create),
        };
        self.nonce += 1;
        *self.provider.send_transaction(req).await.expect("eth_sendTransaction").tx_hash()
    }
}

pub async fn mine(provider: &DynProvider) {
    provider
        .raw_request::<_, serde_json::Value>("evm_mine".into(), Vec::<serde_json::Value>::new())
        .await
        .expect("evm_mine");
}

pub async fn increase_time(provider: &DynProvider, seconds: u64) {
    provider
        .raw_request::<_, serde_json::Value>("evm_increaseTime".into(), (seconds,))
        .await
        .expect("evm_increaseTime");
}

/// Fetch a receipt, polling briefly — automine completes asynchronously
/// after `eth_sendTransaction` returns.
pub async fn receipt_of(provider: &DynProvider, hash: B256) -> TransactionReceipt {
    for _ in 0..100 {
        if let Some(receipt) = provider.get_transaction_receipt(hash).await.expect("receipt fetch")
        {
            return receipt;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("transaction {hash} not mined within 5s");
}

pub async fn balance(provider: &DynProvider, addr: Address) -> U256 {
    provider.get_balance(addr).await.expect("balance")
}

/// Signed decimal string of `post - pre`, matching `SignedAmount::dec`.
pub fn delta_dec(pre: U256, post: U256) -> String {
    if post >= pre { (post - pre).to_string() } else { format!("-{}", pre - post) }
}
