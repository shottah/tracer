//! Trace interpretation: geth `callTracer` normalization and an
//! OpenTracer-style struct-log interpreter.

pub mod calltracer;
pub mod merge;
pub mod structlog;

pub use calltracer::{assign_log_indices, normalize_call_frame};
pub use merge::merge_deep_into;
pub use structlog::{InterpretOutcome, RootCall, interpret_struct_logs};
