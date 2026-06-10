//! Amount types that serialize in UI-friendly triplicate: raw hex, decimal
//! string, and (once token metadata is known) a decimals-formatted string.

use alloy_primitives::U256;
use serde::{Deserialize, Serialize};

/// An unsigned amount of an asset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    /// Raw value, hex-encoded in JSON (`"0x..."`).
    pub raw: U256,
    /// Decimal string of `raw` (U256 does not fit in JSON numbers).
    pub dec: String,
    /// Human form with token decimals applied, e.g. `"1.5"`. Set by enrichment.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub formatted: Option<String>,
}

impl Amount {
    pub fn new(raw: U256) -> Self {
        Self { raw, dec: raw.to_string(), formatted: None }
    }

    pub fn format_with(&mut self, decimals: u8) {
        self.formatted = Some(format_units(self.raw, decimals));
    }
}

impl From<U256> for Amount {
    fn from(raw: U256) -> Self {
        Self::new(raw)
    }
}

/// A signed amount (sign-magnitude, since deltas can exceed `i128`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedAmount {
    /// Magnitude, hex-encoded in JSON.
    pub raw: U256,
    /// `true` when the value is negative. Zero is non-negative.
    pub negative: bool,
    /// Signed decimal string, e.g. `"-1000"`.
    pub dec: String,
    /// Signed human form with decimals applied. Set by enrichment.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub formatted: Option<String>,
}

impl SignedAmount {
    pub const fn zero() -> Self {
        Self { raw: U256::ZERO, negative: false, dec: String::new(), formatted: None }
    }

    pub fn is_zero(&self) -> bool {
        self.raw.is_zero()
    }

    /// Add a positive contribution.
    pub fn add(&mut self, v: U256) {
        if self.negative {
            if self.raw > v {
                self.raw -= v;
            } else {
                self.raw = v - self.raw;
                self.negative = false;
            }
        } else {
            self.raw = self.raw.saturating_add(v);
        }
        self.normalize();
    }

    /// Subtract (a negative contribution).
    pub fn sub(&mut self, v: U256) {
        if self.negative {
            self.raw = self.raw.saturating_add(v);
        } else if self.raw >= v {
            self.raw -= v;
        } else {
            self.raw = v - self.raw;
            self.negative = true;
        }
        self.normalize();
    }

    pub fn format_with(&mut self, decimals: u8) {
        self.formatted = Some(format!("{}{}", self.sign_str(), format_units(self.raw, decimals)));
    }

    fn sign_str(&self) -> &'static str {
        if self.negative && !self.raw.is_zero() { "-" } else { "" }
    }

    fn normalize(&mut self) {
        if self.raw.is_zero() {
            self.negative = false;
        }
        self.dec = format!("{}{}", self.sign_str(), self.raw);
    }
}

impl Default for SignedAmount {
    fn default() -> Self {
        let mut s = Self::zero();
        s.normalize();
        s
    }
}

/// Format `v` with `decimals` fractional digits, trimming trailing zeros
/// (`1500000000000000000, 18` -> `"1.5"`).
pub fn format_units(v: U256, decimals: u8) -> String {
    if decimals == 0 {
        return v.to_string();
    }
    let s = v.to_string();
    let d = decimals as usize;
    let padded = if s.len() <= d { format!("{}{}", "0".repeat(d + 1 - s.len()), s) } else { s };
    let (int, frac) = padded.split_at(padded.len() - d);
    let frac = frac.trim_end_matches('0');
    if frac.is_empty() { int.to_string() } else { format!("{int}.{frac}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_units_works() {
        let one_half = U256::from(1_500_000_000_000_000_000u128);
        assert_eq!(format_units(one_half, 18), "1.5");
        assert_eq!(format_units(U256::from(5u8), 18), "0.000000000000000005");
        assert_eq!(format_units(U256::from(42u8), 0), "42");
        assert_eq!(format_units(U256::ZERO, 18), "0");
        assert_eq!(format_units(U256::from(1_000_000u32), 6), "1");
    }

    #[test]
    fn signed_amount_arithmetic() {
        let mut a = SignedAmount::default();
        assert_eq!(a.dec, "0");
        a.add(U256::from(100u8));
        a.sub(U256::from(250u16));
        assert!(a.negative);
        assert_eq!(a.dec, "-150");
        a.add(U256::from(150u16));
        assert!(a.is_zero());
        assert!(!a.negative);
        assert_eq!(a.dec, "0");
        a.sub(U256::from(7u8));
        a.sub(U256::from(3u8));
        assert_eq!(a.dec, "-10");
        a.add(U256::from(11u8));
        assert_eq!(a.dec, "1");
    }
}
