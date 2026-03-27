// =============================================================================
// vil_new_http — Thin HTTP Adapter
// =============================================================================
// This is the clean, next-generation HTTP adapter.
// Rather than maintaining its own session fabric, it acts as a thin protocol
// mapper over vil_rt::session (core reactive primitives).
// =============================================================================

pub mod source;
pub mod sink;

pub mod format;
pub use format::HttpFormat;

pub use source::{HttpSource, HttpSourceBuilder, FromStreamData, WorkflowBuilderExt, SseSourceDialect};
pub use sink::{HttpSink, HttpSinkBuilder};
