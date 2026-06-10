//! Provider construction and endpoint hygiene.

use crate::ClientError;
use alloy::providers::{DynProvider, Provider, ProviderBuilder};

/// Connect an HTTP provider and erase its type.
pub fn connect(rpc_url: &str) -> Result<DynProvider, ClientError> {
    let url: url::Url = rpc_url.parse().map_err(|e| ClientError::InvalidUrl(format!("{e}")))?;
    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(ClientError::InvalidUrl(format!(
                "unsupported scheme {other:?} (use http or https)"
            )));
        }
    }
    Ok(ProviderBuilder::new().connect_http(url).erased())
}

/// Scheme + host (+ non-default port) only — RPC URLs routinely embed API
/// keys in the path or query, which must never reach a report.
pub fn redact_endpoint(rpc_url: &str) -> Option<String> {
    let url: url::Url = rpc_url.parse().ok()?;
    let host = url.host_str()?;
    Some(match url.port() {
        Some(p) => format!("{}://{host}:{p}", url.scheme()),
        None => format!("{}://{host}", url.scheme()),
    })
}

/// Heuristic: does this RPC error mean the `debug` namespace is unavailable?
pub fn is_unsupported_error(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    m.contains("method not found")
        || m.contains("method not available")
        || m.contains("not supported")
        || m.contains("unsupported method")
        || m.contains("does not exist/is not available")
        || m.contains("-32601")
        || m.contains("403")
        || m.contains("401")
        || m.contains("trace method")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_credentials() {
        assert_eq!(
            redact_endpoint("https://eth-mainnet.g.alchemy.com/v2/SECRET-KEY").as_deref(),
            Some("https://eth-mainnet.g.alchemy.com")
        );
        assert_eq!(
            redact_endpoint("http://127.0.0.1:8545").as_deref(),
            Some("http://127.0.0.1:8545")
        );
    }

    #[test]
    fn classifies_unsupported() {
        assert!(is_unsupported_error(
            "the method debug_traceTransaction does not exist/is not available"
        ));
        assert!(is_unsupported_error("Method not found"));
        assert!(!is_unsupported_error("execution reverted"));
    }
}
