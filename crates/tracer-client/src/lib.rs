//! IO layer: backends that produce trace artifacts (remote RPC or a local
//! anvil fork), token-metadata enrichment, and report orchestration.

pub mod anvil;
pub mod artifacts;
pub mod enrich;
pub mod error;
pub mod rpc;
pub mod tracer;

pub use error::ClientError;
pub use tracer::{BackendChoice, Tracer, TracerConfig};
