//! # vil_ai_compiler
//!
//! AI pipeline compiler — compile RAG/agent workflows to optimized execution
//! plans at build time.
//!
//! ## Overview
//!
//! This crate provides:
//! - **`PipelineNode`** — the set of node types (Embed, Search, Rerank, Generate, …)
//! - **`PipelineDag`** / **`DagBuilder`** — DAG construction with cycle detection
//! - **`compile()`** — topological sort, parallel-group identification,
//!   transform fusion, redundant-cache elimination
//! - **`execute()`** / **`dry_run()`** — tier-by-tier execution with pluggable handlers
//!
//! ## Quick start
//!
//! ```rust
//! use vil_ai_compiler::dag::DagBuilder;
//! use vil_ai_compiler::node::PipelineNode;
//! use vil_ai_compiler::compiler::compile;
//! use vil_ai_compiler::executor::dry_run;
//!
//! let dag = DagBuilder::new()
//!     .node("embed", PipelineNode::Embed { model: "ada".into(), dimensions: 1536 })
//!     .node("search", PipelineNode::Search { index: "docs".into(), top_k: 10 })
//!     .edge("embed", "search")
//!     .build()
//!     .unwrap();
//!
//! let plan = compile(&dag).unwrap();
//! assert_eq!(plan.step_count(), 2);
//!
//! let report = dry_run(&plan);
//! assert!(report.all_ok());
//! ```

pub mod compiler;
pub mod config;
pub mod dag;
pub mod executor;
pub mod node;

pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;

pub use handlers::CompilerStats;
pub use plugin::AiCompilerPlugin;
pub use semantic::{CompileEvent, CompileFault, CompilerState};
