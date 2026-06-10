//! Human-facing renderers for report projections.

pub mod flow;
pub mod table;
pub mod tree;

use tracer_core::{Amount, Asset, TraceReport};

/// `0x1234…cdef` — or the known label when one exists.
pub fn short_addr(report: &TraceReport, addr: alloy_primitives::Address) -> String {
    if let Some(label) = report.address_labels.get(&addr) {
        return label.clone();
    }
    let s = format!("{addr:?}");
    format!("{}…{}", &s[..6], &s[s.len() - 4..])
}

/// Display name for an asset: `ETH`, token symbol, or shortened address;
/// NFTs get `#id` appended.
pub fn asset_label(report: &TraceReport, asset: &Asset) -> String {
    match asset {
        Asset::Native => report.native_symbol.clone(),
        Asset::Erc20 { token } => token_symbol(report, *token),
        Asset::Erc721 { token, token_id } => {
            format!("{} #{token_id}", token_symbol(report, *token))
        }
        Asset::Erc1155 { token, token_id } => {
            format!("{} #{token_id}", token_symbol(report, *token))
        }
    }
}

fn token_symbol(report: &TraceReport, token: alloy_primitives::Address) -> String {
    report
        .tokens
        .get(&token)
        .and_then(|t| t.symbol.clone())
        .unwrap_or_else(|| short_addr(report, token))
}

/// `1.5 WETH` (formatted when decimals known, raw units otherwise).
pub fn amount_display(report: &TraceReport, amount: &Amount, asset: &Asset) -> String {
    let value = amount.formatted.clone().unwrap_or_else(|| amount.dec.clone());
    format!("{value} {}", asset_label(report, asset))
}
