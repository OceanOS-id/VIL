//! MCP (Model Context Protocol) server for VIL VWFD.
//!
//! Provides tools and resources for AI assistants (Claude Desktop, Cursor, etc.)
//! to interact with VIL projects via structured JSON-RPC over stdio.
//!
//! ## Tools
//! - `vil_compile` — compile VWFD YAML
//! - `vil_lint` — validate with VIL Way rules
//! - `vil_list` — list workflows in directory
//! - `vil_explain` — explain lint error code
//! - `vil_scaffold_workflow` — generate workflow stub
//!
//! ## Resources
//! - `vil://workflows/{name}` — read VWFD content
//! - `vil://config` — project config
//!
//! ## Usage
//! ```ignore
//! vil mcp serve
//! // Runs on stdio, JSON-RPC 2.0
//! ```

pub mod protocol;
pub mod tools;
pub mod resources;

pub use protocol::run_server;
