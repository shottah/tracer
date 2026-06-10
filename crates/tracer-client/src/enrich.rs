//! Token metadata enrichment (`symbol`/`name`/`decimals`) via `eth_call`,
//! tolerant of non-standard tokens (bytes32 symbols, missing methods).

use alloy::{
    network::TransactionBuilder,
    providers::{DynProvider, Provider},
    rpc::types::TransactionRequest,
};
use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{SolType, sol_data};
use tracer_core::{TokenInfo, TokenStandard};

const SEL_DECIMALS: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67];
const SEL_SYMBOL: [u8; 4] = [0x95, 0xd8, 0x9b, 0x41];
const SEL_NAME: [u8; 4] = [0x06, 0xfd, 0xde, 0x03];

/// Fill in whatever metadata the token answers; silently leaves fields
/// `None` on failure.
pub async fn fill_token_info(provider: &DynProvider, info: &mut TokenInfo) {
    if info.standard == TokenStandard::Erc20 {
        info.decimals = call(provider, info.address, SEL_DECIMALS)
            .await
            .and_then(|ret| decode_u8(ret.as_ref()));
    }
    info.symbol =
        call(provider, info.address, SEL_SYMBOL).await.and_then(|r| decode_string(r.as_ref()));
    info.name =
        call(provider, info.address, SEL_NAME).await.and_then(|r| decode_string(r.as_ref()));
}

async fn call(provider: &DynProvider, to: Address, selector: [u8; 4]) -> Option<Bytes> {
    let req = TransactionRequest::default().with_to(to).with_input(Bytes::from(selector.to_vec()));
    provider.call(req).await.ok()
}

fn decode_u8(ret: &[u8]) -> Option<u8> {
    let v = sol_data::Uint::<256>::abi_decode(ret).ok()?;
    u8::try_from(v).ok()
}

/// ABI string, with a bytes32 fallback (MKR-style tokens).
fn decode_string(ret: &[u8]) -> Option<String> {
    if let Ok(s) = sol_data::String::abi_decode(ret) {
        let s = s.trim().trim_matches(char::from(0)).to_string();
        return (!s.is_empty()).then_some(s);
    }
    if ret.len() == 32 && U256::from_be_slice(ret) != U256::ZERO {
        let trimmed: Vec<u8> = ret.iter().copied().take_while(|b| *b != 0).collect();
        let s = String::from_utf8(trimmed).ok()?;
        let s = s.trim().to_string();
        return (!s.is_empty()).then_some(s);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_sol_types::SolValue;

    #[test]
    fn decodes_standard_and_bytes32_strings() {
        let abi = "WETH".to_string().abi_encode();
        assert_eq!(decode_string(&abi).as_deref(), Some("WETH"));

        let mut b32 = [0u8; 32];
        b32[..3].copy_from_slice(b"MKR");
        assert_eq!(decode_string(&b32).as_deref(), Some("MKR"));

        assert_eq!(decode_string(&[0u8; 32]), None);
        assert_eq!(decode_string(&[]), None);
    }

    #[test]
    fn decodes_decimals() {
        let enc = U256::from(18u8).abi_encode();
        assert_eq!(decode_u8(&enc), Some(18));
        assert_eq!(decode_u8(&U256::from(300u16).abi_encode()), None);
    }
}
