//! Selector and event decoding for token standards and common protocol entry
//! points. Pure and offline: a built-in table plus `sol!`-generated ABI
//! decoding for the calls that matter to transfer semantics.

use crate::trace::{DecodedCall, DecodedEvent, DecodedParam};
use alloy_primitives::{Address, B256, U256, hex};
use alloy_sol_types::{SolCall, SolEvent, sol};

sol! {
    /// Minimal ERC-20 surface.
    interface IERC20 {
        event Transfer(address indexed from, address indexed to, uint256 value);
        event Approval(address indexed owner, address indexed spender, uint256 value);
        function transfer(address to, uint256 amount) external returns (bool);
        function transferFrom(address from, address to, uint256 amount) external returns (bool);
        function approve(address spender, uint256 amount) external returns (bool);
        function balanceOf(address owner) external view returns (uint256);
        function decimals() external view returns (uint8);
        function symbol() external view returns (string);
        function name() external view returns (string);
    }

    interface IERC721 {
        event ApprovalForAll(address indexed owner, address indexed operator, bool approved);
        function safeTransferFrom(address from, address to, uint256 tokenId) external;
        function setApprovalForAll(address operator, bool approved) external;
    }

    interface IERC1155 {
        event TransferSingle(address indexed operator, address indexed from, address indexed to, uint256 id, uint256 value);
        event TransferBatch(address indexed operator, address indexed from, address indexed to, uint256[] ids, uint256[] values);
        function safeTransferFrom(address from, address to, uint256 id, uint256 amount, bytes data) external;
        function safeBatchTransferFrom(address from, address to, uint256[] ids, uint256[] amounts, bytes data) external;
    }

    /// WETH-style wrapper events.
    interface IWrappedNative {
        event Deposit(address indexed dst, uint256 wad);
        event Withdrawal(address indexed src, uint256 wad);
        function deposit() external payable;
        function withdraw(uint256 wad) external;
    }
}

/// A token-standard event extracted from a raw log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenEvent {
    Erc20Transfer {
        token: Address,
        from: Address,
        to: Address,
        value: U256,
    },
    Erc721Transfer {
        token: Address,
        from: Address,
        to: Address,
        token_id: U256,
    },
    Erc1155Single {
        token: Address,
        from: Address,
        to: Address,
        token_id: U256,
        value: U256,
    },
    Erc1155Batch {
        token: Address,
        from: Address,
        to: Address,
        ids: Vec<U256>,
        values: Vec<U256>,
    },
    /// Wrapped-native mint.
    Deposit {
        token: Address,
        dst: Address,
        wad: U256,
    },
    /// Wrapped-native burn.
    Withdrawal {
        token: Address,
        src: Address,
        wad: U256,
    },
}

fn topic_addr(t: B256) -> Address {
    Address::from_word(t)
}

fn data_word(data: &[u8], i: usize) -> Option<U256> {
    data.get(i * 32..(i + 1) * 32).map(U256::from_be_slice)
}

/// Decode a raw log into a [`TokenEvent`], if it is one.
///
/// ERC-20 and ERC-721 share the `Transfer` topic0 and are disambiguated by
/// indexed-topic count (3 vs 4), per the standards.
pub fn decode_token_event(address: Address, topics: &[B256], data: &[u8]) -> Option<TokenEvent> {
    let t0 = *topics.first()?;
    if t0 == IERC20::Transfer::SIGNATURE_HASH {
        match topics.len() {
            3 => {
                return Some(TokenEvent::Erc20Transfer {
                    token: address,
                    from: topic_addr(topics[1]),
                    to: topic_addr(topics[2]),
                    value: data_word(data, 0)?,
                });
            }
            4 => {
                return Some(TokenEvent::Erc721Transfer {
                    token: address,
                    from: topic_addr(topics[1]),
                    to: topic_addr(topics[2]),
                    token_id: U256::from_be_slice(topics[3].as_slice()),
                });
            }
            _ => return None,
        }
    }
    if t0 == IERC1155::TransferSingle::SIGNATURE_HASH && topics.len() == 4 {
        return Some(TokenEvent::Erc1155Single {
            token: address,
            from: topic_addr(topics[2]),
            to: topic_addr(topics[3]),
            token_id: data_word(data, 0)?,
            value: data_word(data, 1)?,
        });
    }
    if t0 == IERC1155::TransferBatch::SIGNATURE_HASH && topics.len() == 4 {
        let ev = IERC1155::TransferBatch::decode_raw_log(topics.iter().copied(), data).ok()?;
        return Some(TokenEvent::Erc1155Batch {
            token: address,
            from: ev.from,
            to: ev.to,
            ids: ev.ids,
            values: ev.values,
        });
    }
    if t0 == IWrappedNative::Deposit::SIGNATURE_HASH && topics.len() == 2 {
        return Some(TokenEvent::Deposit {
            token: address,
            dst: topic_addr(topics[1]),
            wad: data_word(data, 0)?,
        });
    }
    if t0 == IWrappedNative::Withdrawal::SIGNATURE_HASH && topics.len() == 2 {
        return Some(TokenEvent::Withdrawal {
            token: address,
            src: topic_addr(topics[1]),
            wad: data_word(data, 0)?,
        });
    }
    None
}

fn param(name: &str, value: impl ToString) -> DecodedParam {
    DecodedParam { name: name.into(), value: value.to_string() }
}

/// Human-readable decoding of a log for display purposes.
pub fn decode_event(address: Address, topics: &[B256], data: &[u8]) -> Option<DecodedEvent> {
    if let Some(ev) = decode_token_event(address, topics, data) {
        return Some(match ev {
            TokenEvent::Erc20Transfer { from, to, value, .. } => DecodedEvent {
                name: "Transfer".into(),
                signature: "Transfer(address,address,uint256)".into(),
                params: vec![param("from", from), param("to", to), param("value", value)],
            },
            TokenEvent::Erc721Transfer { from, to, token_id, .. } => DecodedEvent {
                name: "Transfer".into(),
                signature: "Transfer(address,address,uint256)".into(),
                params: vec![param("from", from), param("to", to), param("tokenId", token_id)],
            },
            TokenEvent::Erc1155Single { from, to, token_id, value, .. } => DecodedEvent {
                name: "TransferSingle".into(),
                signature: "TransferSingle(address,address,address,uint256,uint256)".into(),
                params: vec![
                    param("from", from),
                    param("to", to),
                    param("id", token_id),
                    param("value", value),
                ],
            },
            TokenEvent::Erc1155Batch { from, to, ids, values, .. } => DecodedEvent {
                name: "TransferBatch".into(),
                signature: "TransferBatch(address,address,address,uint256[],uint256[])".into(),
                params: vec![
                    param("from", from),
                    param("to", to),
                    param("ids", format!("[{} ids]", ids.len())),
                    param("values", format!("[{} values]", values.len())),
                ],
            },
            TokenEvent::Deposit { dst, wad, .. } => DecodedEvent {
                name: "Deposit".into(),
                signature: "Deposit(address,uint256)".into(),
                params: vec![param("dst", dst), param("wad", wad)],
            },
            TokenEvent::Withdrawal { src, wad, .. } => DecodedEvent {
                name: "Withdrawal".into(),
                signature: "Withdrawal(address,uint256)".into(),
                params: vec![param("src", src), param("wad", wad)],
            },
        });
    }
    let t0 = *topics.first()?;
    if t0 == IERC20::Approval::SIGNATURE_HASH && topics.len() == 3 {
        return Some(DecodedEvent {
            name: "Approval".into(),
            signature: "Approval(address,address,uint256)".into(),
            params: vec![
                param("owner", topic_addr(topics[1])),
                param("spender", topic_addr(topics[2])),
                param("value", data_word(data, 0).unwrap_or_default()),
            ],
        });
    }
    if t0 == IERC721::ApprovalForAll::SIGNATURE_HASH && topics.len() == 3 {
        return Some(DecodedEvent {
            name: "ApprovalForAll".into(),
            signature: "ApprovalForAll(address,address,bool)".into(),
            params: vec![
                param("owner", topic_addr(topics[1])),
                param("operator", topic_addr(topics[2])),
                param("approved", data_word(data, 0).map(|v| !v.is_zero()).unwrap_or_default()),
            ],
        });
    }
    None
}

/// Well-known selectors decoded by name only (no argument decoding).
/// Signatures here are display labels; only add entries with verified hashes.
const KNOWN_SIGNATURES: &[([u8; 4], &str)] = &[
    ([0x70, 0xa0, 0x82, 0x31], "balanceOf(address)"),
    ([0xdd, 0x62, 0xed, 0x3e], "allowance(address,address)"),
    ([0x18, 0x16, 0x0d, 0xdd], "totalSupply()"),
    ([0x31, 0x3c, 0xe5, 0x67], "decimals()"),
    ([0x95, 0xd8, 0x9b, 0x41], "symbol()"),
    ([0x06, 0xfd, 0xde, 0x03], "name()"),
    ([0x40, 0xc1, 0x0f, 0x19], "mint(address,uint256)"),
    ([0xa0, 0x71, 0x2d, 0x68], "mint(uint256)"),
    ([0x12, 0x49, 0xc5, 0x8b], "mint()"),
    ([0x42, 0x96, 0x6c, 0x68], "burn(uint256)"),
    ([0x3c, 0xcf, 0xd6, 0x0b], "withdraw()"),
    ([0xb8, 0x8d, 0x4f, 0xde], "safeTransferFrom(address,address,uint256,bytes)"),
    ([0x63, 0x52, 0x21, 0x1e], "ownerOf(uint256)"),
    ([0x08, 0x18, 0x12, 0xfc], "getApproved(uint256)"),
    ([0xe9, 0x85, 0xe9, 0xc5], "isApprovedForAll(address,address)"),
    ([0x01, 0xff, 0xc9, 0xa7], "supportsInterface(bytes4)"),
    ([0xac, 0x96, 0x50, 0xd8], "multicall(bytes[])"),
    ([0x5a, 0xe4, 0x01, 0xdc], "multicall(uint256,bytes[])"),
    ([0x35, 0x93, 0x56, 0x4c], "execute(bytes,bytes[],uint256)"),
    ([0x24, 0x85, 0x6b, 0xc3], "execute(bytes,bytes[])"),
    (
        [0x38, 0xed, 0x17, 0x39],
        "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)",
    ),
    (
        [0x88, 0x03, 0xdb, 0xee],
        "swapTokensForExactTokens(uint256,uint256,address[],address,uint256)",
    ),
    ([0x7f, 0xf3, 0x6a, 0xb5], "swapExactETHForTokens(uint256,address[],address,uint256)"),
    ([0x18, 0xcb, 0xaf, 0xe5], "swapExactTokensForETH(uint256,uint256,address[],address,uint256)"),
    (
        [0x5c, 0x11, 0xd7, 0x95],
        "swapExactTokensForTokensSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)",
    ),
    (
        [0x79, 0x1a, 0xc9, 0x47],
        "swapExactTokensForETHSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)",
    ),
    (
        [0xb6, 0xf9, 0xde, 0x95],
        "swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256)",
    ),
    ([0x02, 0x2c, 0x0d, 0x9f], "swap(uint256,uint256,address,bytes)"),
    ([0x12, 0x8a, 0xcb, 0x08], "swap(address,bool,int256,uint160,bytes)"),
    (
        [0x41, 0x4b, 0xf3, 0x89],
        "exactInputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))",
    ),
    (
        [0x04, 0xe4, 0x5a, 0xaf],
        "exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))",
    ),
    ([0xc0, 0x4b, 0x8d, 0x59], "exactInput((bytes,address,uint256,uint256,uint256))"),
    ([0xb8, 0x58, 0x18, 0x3f], "exactInput((bytes,address,uint256,uint256))"),
    ([0xfa, 0x46, 0x1e, 0x33], "uniswapV3SwapCallback(int256,int256,bytes)"),
    ([0xd5, 0x05, 0xac, 0xcf], "permit(address,address,uint256,uint256,uint8,bytes32,bytes32)"),
    ([0x09, 0x02, 0xf1, 0xac], "getReserves()"),
    ([0x0d, 0xfe, 0x16, 0x81], "token0()"),
    ([0xd2, 0x12, 0x20, 0xa7], "token1()"),
    (
        [0xe8, 0xe3, 0x37, 0x00],
        "addLiquidity(address,address,uint256,uint256,uint256,uint256,address,uint256)",
    ),
    ([0xf3, 0x05, 0xd7, 0x19], "addLiquidityETH(address,uint256,uint256,uint256,address,uint256)"),
    (
        [0xba, 0xa2, 0xab, 0xde],
        "removeLiquidity(address,address,uint256,uint256,uint256,address,uint256)",
    ),
    ([0x15, 0x0b, 0x7a, 0x02], "onERC721Received(address,address,uint256,bytes)"),
    ([0xf2, 0x3a, 0x6e, 0x61], "onERC1155Received(address,address,uint256,uint256,bytes)"),
    ([0xbc, 0x19, 0x7c, 0x81], "onERC1155BatchReceived(address,address,uint256[],uint256[],bytes)"),
    ([0x8d, 0xa5, 0xcb, 0x5b], "owner()"),
    ([0xf2, 0xfd, 0xe3, 0x8b], "transferOwnership(address)"),
    ([0x36, 0x44, 0xe5, 0x15], "DOMAIN_SEPARATOR()"),
    ([0x7e, 0xce, 0xbe, 0x00], "nonces(address)"),
];

fn name_of(signature: &str) -> String {
    signature.split('(').next().unwrap_or(signature).to_string()
}

/// Decode standard revert payloads: `Error(string)` and `Panic(uint256)`.
pub fn revert_reason(output: &[u8]) -> Option<String> {
    use alloy_sol_types::{SolType, sol_data};
    if output.len() < 4 {
        return None;
    }
    match [output[0], output[1], output[2], output[3]] {
        // Error(string)
        [0x08, 0xc3, 0x79, 0xa0] => sol_data::String::abi_decode(&output[4..]).ok(),
        // Panic(uint256)
        [0x4e, 0x48, 0x7b, 0x71] => {
            let code = sol_data::Uint::<256>::abi_decode(&output[4..]).ok()?;
            Some(format!("panic: 0x{code:x}"))
        }
        _ => None,
    }
}

/// Best-effort decoding of call input. Returns `None` for inputs shorter than
/// a selector; otherwise always returns at least the selector.
pub fn decode_call(input: &[u8]) -> Option<DecodedCall> {
    if input.len() < 4 {
        return None;
    }
    let sel: [u8; 4] = input[..4].try_into().ok()?;
    let mut dc = DecodedCall {
        selector: format!("0x{}", hex::encode(sel)),
        name: None,
        signature: None,
        params: Vec::new(),
    };

    let mut set = |name: &str, sig: &str, params: Vec<DecodedParam>| {
        dc.name = Some(name.into());
        dc.signature = Some(sig.into());
        dc.params = params;
    };

    match sel {
        s if s == IERC20::transferCall::SELECTOR => {
            if let Ok(c) = IERC20::transferCall::abi_decode(input) {
                set(
                    "transfer",
                    "transfer(address,uint256)",
                    vec![param("to", c.to), param("amount", c.amount)],
                );
            }
        }
        s if s == IERC20::transferFromCall::SELECTOR => {
            if let Ok(c) = IERC20::transferFromCall::abi_decode(input) {
                set(
                    "transferFrom",
                    "transferFrom(address,address,uint256)",
                    vec![param("from", c.from), param("to", c.to), param("amountOrId", c.amount)],
                );
            }
        }
        s if s == IERC20::approveCall::SELECTOR => {
            if let Ok(c) = IERC20::approveCall::abi_decode(input) {
                set(
                    "approve",
                    "approve(address,uint256)",
                    vec![param("spender", c.spender), param("amount", c.amount)],
                );
            }
        }
        s if s == IERC721::safeTransferFromCall::SELECTOR => {
            if let Ok(c) = IERC721::safeTransferFromCall::abi_decode(input) {
                set(
                    "safeTransferFrom",
                    "safeTransferFrom(address,address,uint256)",
                    vec![param("from", c.from), param("to", c.to), param("tokenId", c.tokenId)],
                );
            }
        }
        s if s == IERC721::setApprovalForAllCall::SELECTOR => {
            if let Ok(c) = IERC721::setApprovalForAllCall::abi_decode(input) {
                set(
                    "setApprovalForAll",
                    "setApprovalForAll(address,bool)",
                    vec![param("operator", c.operator), param("approved", c.approved)],
                );
            }
        }
        s if s == IERC1155::safeTransferFromCall::SELECTOR => {
            if let Ok(c) = IERC1155::safeTransferFromCall::abi_decode(input) {
                set(
                    "safeTransferFrom",
                    "safeTransferFrom(address,address,uint256,uint256,bytes)",
                    vec![
                        param("from", c.from),
                        param("to", c.to),
                        param("id", c.id),
                        param("amount", c.amount),
                    ],
                );
            }
        }
        s if s == IERC1155::safeBatchTransferFromCall::SELECTOR => {
            if let Ok(c) = IERC1155::safeBatchTransferFromCall::abi_decode(input) {
                set(
                    "safeBatchTransferFrom",
                    "safeBatchTransferFrom(address,address,uint256[],uint256[],bytes)",
                    vec![
                        param("from", c.from),
                        param("to", c.to),
                        param("ids", format!("[{} ids]", c.ids.len())),
                    ],
                );
            }
        }
        s if s == IWrappedNative::depositCall::SELECTOR => {
            set("deposit", "deposit()", Vec::new());
        }
        s if s == IWrappedNative::withdrawCall::SELECTOR => {
            if let Ok(c) = IWrappedNative::withdrawCall::abi_decode(input) {
                set("withdraw", "withdraw(uint256)", vec![param("wad", c.wad)]);
            }
        }
        _ => {}
    }

    if dc.name.is_none()
        && let Some((_, sig)) = KNOWN_SIGNATURES.iter().find(|(s, _)| *s == sel)
    {
        dc.name = Some(name_of(sig));
        dc.signature = Some((*sig).into());
    }
    Some(dc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256, bytes};
    use alloy_sol_types::SolValue;

    const TOKEN: Address = address!("0x00000000000000000000000000000000000a11ce");
    const A: Address = address!("0x1000000000000000000000000000000000000001");
    const B: Address = address!("0x2000000000000000000000000000000000000002");

    #[test]
    fn transfer_topic0_is_canonical() {
        // keccak256("Transfer(address,address,uint256)")
        assert_eq!(
            IERC20::Transfer::SIGNATURE_HASH,
            b256!("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")
        );
    }

    #[test]
    fn decodes_erc20_transfer_event() {
        let topics = [IERC20::Transfer::SIGNATURE_HASH, A.into_word(), B.into_word()];
        let data = U256::from(5u8).abi_encode();
        let ev = decode_token_event(TOKEN, &topics, &data).unwrap();
        assert_eq!(
            ev,
            TokenEvent::Erc20Transfer { token: TOKEN, from: A, to: B, value: U256::from(5u8) }
        );
    }

    #[test]
    fn disambiguates_erc721_by_topic_count() {
        let topics = [
            IERC20::Transfer::SIGNATURE_HASH,
            A.into_word(),
            B.into_word(),
            B256::from(U256::from(7u8)),
        ];
        let ev = decode_token_event(TOKEN, &topics, &[]).unwrap();
        assert_eq!(
            ev,
            TokenEvent::Erc721Transfer { token: TOKEN, from: A, to: B, token_id: U256::from(7u8) }
        );
    }

    #[test]
    fn decodes_erc1155_batch() {
        let ids = vec![U256::from(1u8), U256::from(2u8)];
        let values = vec![U256::from(10u8), U256::from(20u8)];
        let topics = [
            IERC1155::TransferBatch::SIGNATURE_HASH,
            A.into_word(), // operator
            A.into_word(), // from
            B.into_word(), // to
        ];
        let data = (ids.clone(), values.clone()).abi_encode_params();
        let ev = decode_token_event(TOKEN, &topics, &data).unwrap();
        assert_eq!(ev, TokenEvent::Erc1155Batch { token: TOKEN, from: A, to: B, ids, values });
    }

    #[test]
    fn decodes_transfer_call() {
        let call = IERC20::transferCall { to: B, amount: U256::from(1234u16) };
        let input = call.abi_encode();
        let dc = decode_call(&input).unwrap();
        assert_eq!(dc.selector, "0xa9059cbb");
        assert_eq!(dc.name.as_deref(), Some("transfer"));
        assert_eq!(dc.params[1].value, "1234");
    }

    #[test]
    fn unknown_selector_keeps_selector() {
        let dc = decode_call(&bytes!("0xdeadbeef00")).unwrap();
        assert_eq!(dc.selector, "0xdeadbeef");
        assert!(dc.name.is_none());
        assert!(decode_call(&[0x01]).is_none());
    }

    #[test]
    fn known_signature_lookup() {
        let dc = decode_call(&bytes!("0xac9650d8")).unwrap();
        assert_eq!(dc.name.as_deref(), Some("multicall"));
    }
}
