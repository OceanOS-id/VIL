//! # VIL — Pythonic Rust Framework
//!
//! Zero-copy, high-performance backend framework with batteries included.
//!
//! ## Quick Start
//!
//! ```toml
//! [dependencies]
//! vil = { version = "0.2", features = ["web", "db-sqlite"] }
//! ```
//!
//! ```rust,ignore
//! use vil::prelude::*;
//!
//! #[vil_handler]
//! async fn hello() -> VilResponse<&'static str> {
//!     VilResponse::ok("Hello VIL!")
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let svc = ServiceProcess::new("hello")
//!         .endpoint(Method::GET, "/", get(hello));
//!     VilApp::new("my-app")
//!         .port(8082)
//!         .observer(true)
//!         .service(svc)
//!         .run().await;
//! }
//! ```

/// Common re-exports — `use vil::prelude::*` gives everything you need.
pub mod prelude {
    // Server framework
    #[cfg(feature = "web")]
    pub use vil_server::prelude::*;

    // Auth
    #[cfg(feature = "web")]
    pub use vil_server_auth::{VilPassword, VilJwt, VilClaims, TokenPair};

    // Logging
    #[cfg(feature = "log")]
    pub use vil_log;
}

// ── Crate re-exports ──

#[cfg(feature = "web")]
pub use vil_server;
#[cfg(feature = "web")]
pub use vil_server_core;
#[cfg(feature = "web")]
pub use vil_server_auth;
#[cfg(feature = "web")]
pub use vil_server_auth as auth;
#[cfg(feature = "web")]
pub use vil_server_web as web;
#[cfg(feature = "web")]
pub use vil_json;

#[cfg(feature = "log")]
pub use vil_log;

#[cfg(feature = "sdk")]
pub use vil_sdk;

// Database
#[cfg(feature = "db-sqlite")]
pub use vil_db_sqlx;
#[cfg(feature = "db-postgres")]
pub use vil_db_sqlx;
#[cfg(feature = "db-sqlite")]
pub use vil_db_semantic;
#[cfg(feature = "db-postgres")]
pub use vil_db_semantic;
#[cfg(feature = "db-redis")]
pub use vil_db_redis;

// AI
#[cfg(feature = "ai")]
pub mod ai {
    pub use vil_llm::*;
    pub use vil_ai_gateway::*;
    pub use vil_ai_trace::*;
    pub use vil_cost_tracker::*;
    pub use vil_prompts::*;
    pub use vil_prompt_shield::*;
    pub use vil_output_parser::*;
    pub use vil_guardrails::*;
}

// Infrastructure
#[cfg(feature = "ws")]
pub use vil_ws;
#[cfg(feature = "cache")]
pub use vil_cache;
