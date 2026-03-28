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

pub mod builder;
pub mod registry;
pub mod template;

// Re-exports
pub use builder::{code_review_template, rag_qa_template, summarize_template, PromptBuilder};
pub use registry::PromptRegistry;
pub use template::{PromptError, PromptTemplate};

// VIL integration layer
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;

pub use plugin::PromptsPlugin;
pub use semantic::{PromptFault, PromptRenderEvent, PromptsState};
