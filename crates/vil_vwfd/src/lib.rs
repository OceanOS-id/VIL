//! # vil_vwfd — VIL VWFD Runtime
//!
//! Compile and execute VWFD workflows on VIL infrastructure.
//! Same VWFD YAML format as VFlow — different runtime, own compiler.

pub mod spec;
pub mod graph;
pub mod compiler;
pub mod spv1;
pub mod eval_bridge;
pub mod executor;
pub mod triggers;
pub mod durability;
pub mod saga;
pub mod handler;
pub mod loader;
pub mod process;
pub mod cli;
pub mod mcp;
pub mod registry;
pub mod app;

pub use compiler::compile;
pub use graph::{VilwGraph, NodeKind};
pub use executor::{execute, ExecConfig, ExecResult, ExecError};
pub use durability::DurabilityStore;
pub use saga::{collect_compensations, run_compensation};
pub use handler::WorkflowRouter;
pub use loader::{load_dir, load_yaml};
pub use app::{app, VwfdApp, NativeRegistry};
