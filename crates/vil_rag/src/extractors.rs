//! Convenience extractors for RAG handler injection.
//!
//! Instead of writing `Extension<Arc<RagPipeline>>`, use the `Rag`
//! newtype which implements `Deref<Target = RagPipeline>`.
//!
//! # Example
//!
//! ```rust,ignore
//! use vil_rag::extractors::Rag;
//! use axum::extract::Extension;
//!
//! async fn handler(Extension(rag): Extension<Rag>) -> impl IntoResponse {
//!     let result = rag.query("What is Rust?").await?;
//!     // ...
//! }
//! ```

use std::ops::Deref;
use std::sync::Arc;

use crate::pipeline::RagPipeline;

// ---------------------------------------------------------------------------
// Rag — newtype wrapper for Arc<RagPipeline>
// ---------------------------------------------------------------------------

/// Convenience wrapper around `Arc<RagPipeline>` for handler injection.
///
/// Use with `Extension<Rag>` in handler parameters.  Derefs to `RagPipeline`
/// so you can call `.query()`, `.ingest()`, etc. directly.
#[derive(Clone)]
pub struct Rag(pub Arc<RagPipeline>);

impl Deref for Rag {
    type Target = RagPipeline;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<RagPipeline>> for Rag {
    fn from(inner: Arc<RagPipeline>) -> Self {
        Rag(inner)
    }
}

impl Rag {
    /// Wrap an existing `Arc<RagPipeline>`.
    pub fn new(pipeline: Arc<RagPipeline>) -> Self {
        Rag(pipeline)
    }

    /// Get the inner `Arc<RagPipeline>`.
    pub fn inner(&self) -> &Arc<RagPipeline> {
        &self.0
    }
}
