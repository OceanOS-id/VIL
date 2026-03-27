//! Convenience extractors for Agent handler injection.
//!
//! Instead of writing `Extension<Arc<Agent>>`, use the `AgentHandle`
//! newtype which implements `Deref<Target = Agent>`.
//!
//! # Example
//!
//! ```rust,ignore
//! use vil_agent::extractors::AgentHandle;
//! use axum::extract::Extension;
//!
//! async fn handler(Extension(agent): Extension<AgentHandle>) -> impl IntoResponse {
//!     let response = agent.run("What is 2+2?").await?;
//!     // ...
//! }
//! ```

use std::ops::Deref;
use std::sync::Arc;

use crate::agent::Agent;

// ---------------------------------------------------------------------------
// AgentHandle — newtype wrapper for Arc<Agent>
// ---------------------------------------------------------------------------

/// Convenience wrapper around `Arc<Agent>` for handler injection.
///
/// Use with `Extension<AgentHandle>` in handler parameters.  Derefs to `Agent`
/// so you can call `.run()` directly.
#[derive(Clone)]
pub struct AgentHandle(pub Arc<Agent>);

impl Deref for AgentHandle {
    type Target = Agent;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<Agent>> for AgentHandle {
    fn from(inner: Arc<Agent>) -> Self {
        AgentHandle(inner)
    }
}

impl AgentHandle {
    /// Wrap an existing `Arc<Agent>`.
    pub fn new(agent: Arc<Agent>) -> Self {
        AgentHandle(agent)
    }

    /// Get the inner `Arc<Agent>`.
    pub fn inner(&self) -> &Arc<Agent> {
        &self.0
    }
}
