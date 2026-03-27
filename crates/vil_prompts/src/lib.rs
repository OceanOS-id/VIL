//! # vil_prompts (H04)
//!
//! Prompt Template Engine for VIL.
//!
//! Provides compile-time validated prompt templates with a fluent builder API,
//! a named template registry, and pre-built templates for common AI tasks.
//!
//! ## Quick start
//!
//! ```rust
//! use vil_prompts::{PromptTemplate, PromptBuilder, PromptRegistry};
//! use std::collections::HashMap;
//!
//! // Direct template usage
//! let tpl = PromptTemplate::new("Hello, {name}!").unwrap();
//! let mut vars = HashMap::new();
//! vars.insert("name".to_string(), "World".to_string());
//! assert_eq!(tpl.render(&vars).unwrap(), "Hello, World!");
//!
//! // Builder pattern
//! let tpl = PromptBuilder::new()
//!     .system("You are helpful.")
//!     .user("{question}")
//!     .build()
//!     .unwrap();
//! ```

pub mod template;
pub mod registry;
pub mod builder;

// Re-exports
pub use template::{PromptTemplate, PromptError};
pub use registry::PromptRegistry;
pub use builder::{PromptBuilder, rag_qa_template, summarize_template, code_review_template};

// VIL integration layer
pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::PromptsPlugin;
pub use semantic::{PromptRenderEvent, PromptFault, PromptsState};
