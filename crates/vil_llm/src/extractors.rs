//! Convenience extractors for LLM handler injection.
//!
//! Instead of writing `Extension<Arc<dyn LlmProvider>>`, use the `Llm`
//! newtype which implements `Deref<Target = dyn LlmProvider>`.
//!
//! # Example
//!
//! ```rust,ignore
//! use vil_llm::extractors::Llm;
//! use axum::extract::Extension;
//!
//! async fn handler(Extension(llm): Extension<Llm>) -> impl IntoResponse {
//!     let response = llm.chat(&messages).await?;
//!     // ...
//! }
//! ```

use std::ops::Deref;
use std::sync::Arc;

use crate::provider::{EmbeddingProvider, LlmProvider};

// ---------------------------------------------------------------------------
// Llm — newtype wrapper for Arc<dyn LlmProvider>
// ---------------------------------------------------------------------------

/// Convenience wrapper around `Arc<dyn LlmProvider>` for handler injection.
///
/// Use with `Extension<Llm>` in handler parameters.  Derefs to `dyn LlmProvider`
/// so you can call `.chat()`, `.model()`, etc. directly.
#[derive(Clone)]
pub struct Llm(pub Arc<dyn LlmProvider>);

impl Deref for Llm {
    type Target = dyn LlmProvider;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl From<Arc<dyn LlmProvider>> for Llm {
    fn from(inner: Arc<dyn LlmProvider>) -> Self {
        Llm(inner)
    }
}

impl Llm {
    /// Create a new `Llm` wrapper from any `LlmProvider` implementation.
    pub fn new(provider: impl LlmProvider + 'static) -> Self {
        Llm(Arc::new(provider))
    }

    /// Get the inner `Arc<dyn LlmProvider>`.
    pub fn inner(&self) -> &Arc<dyn LlmProvider> {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// Embedder — newtype wrapper for Arc<dyn EmbeddingProvider>
// ---------------------------------------------------------------------------

/// Convenience wrapper around `Arc<dyn EmbeddingProvider>` for handler injection.
///
/// Use with `Extension<Embedder>` in handler parameters.  Derefs to
/// `dyn EmbeddingProvider` so you can call `.embed()`, `.dimension()`, etc.
#[derive(Clone)]
pub struct Embedder(pub Arc<dyn EmbeddingProvider>);

impl Deref for Embedder {
    type Target = dyn EmbeddingProvider;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl From<Arc<dyn EmbeddingProvider>> for Embedder {
    fn from(inner: Arc<dyn EmbeddingProvider>) -> Self {
        Embedder(inner)
    }
}

impl Embedder {
    /// Create a new `Embedder` wrapper from any `EmbeddingProvider` implementation.
    pub fn new(provider: impl EmbeddingProvider + 'static) -> Self {
        Embedder(Arc::new(provider))
    }

    /// Get the inner `Arc<dyn EmbeddingProvider>`.
    pub fn inner(&self) -> &Arc<dyn EmbeddingProvider> {
        &self.0
    }
}
