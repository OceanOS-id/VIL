//! # VIL SQL Query Generator (I05)
//!
//! Text-to-SQL patterns, schema registry, injection prevention, and safe query
//! generation. No database driver dependency — this crate generates SQL strings
//! and validates them.
//!
//! ## Quick Start
//!
//! ```rust
//! use vil_sql_agent::{SchemaRegistry, TableSchema, SqlGenerator, validate_sql, Severity};
//!
//! let mut reg = SchemaRegistry::new();
//! reg.register(
//!     TableSchema::new("users")
//!         .column("id", "INTEGER", false, true)
//!         .column("name", "VARCHAR(255)", false, false),
//! );
//!
//! let gen = SqlGenerator::new(&reg);
//! let query = gen.select_all("users").unwrap();
//! assert!(query.safe);
//!
//! let danger = validate_sql("DROP TABLE users");
//! assert_eq!(danger.severity, Severity::Dangerous);
//! ```

pub mod config;
pub mod generator;
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod schema;
pub mod semantic;
pub mod validator;

pub use config::{PlaceholderStyle, SqlAgentConfig};
pub use generator::SqlGenerator;
pub use plugin::SqlAgentPlugin;
pub use schema::{Column, SchemaRegistry, TableSchema};
pub use semantic::{SqlAgentEvent, SqlAgentFault, SqlAgentFaultType, SqlAgentState};
pub use validator::{validate_sql, SafeQuery, Severity, ValidationResult};
