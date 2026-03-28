// =============================================================================
// vil_new_http — Thin HTTP Adapter
// =============================================================================
// This is the clean, next-generation HTTP adapter.
// Rather than maintaining its own session fabric, it acts as a thin protocol
// mapper over vil_rt::session (core reactive primitives).
// =============================================================================

pub mod sink;
pub mod source;

pub mod format;
pub use format::HttpFormat;

pub use sink::{HttpSink, HttpSinkBuilder};
pub use source::{
    FromStreamData, HttpSource, HttpSourceBuilder, SseSourceDialect, WorkflowBuilderExt,
};
