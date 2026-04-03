//! # VilORM — Process-Oriented Zero-Copy ORM
//!
//! Built on sqlx. Provides `#[derive(VilEntity)]` for auto CRUD + query builder.
//!
//! ```ignore
//! use vil_orm::prelude::*;
//!
//! #[derive(VilEntity, VilModel, sqlx::FromRow)]
//! #[vil_entity(table = "todos")]
//! struct Todo {
//!     #[vil_entity(pk, auto_uuid)]
//!     id: String,
//!     title: String,
//!     done: i64,
//! }
//!
//! let todos = Todo::find_all(&pool).await?;
//! Todo::delete(&pool, "some-id").await?;
//! ```

pub mod pagination;
pub mod bind;
pub mod query;
pub mod log;

// Re-export derive macro
pub use vil_orm_derive::VilEntity;
pub use vil_orm_derive::VilCrud;

// Re-export pool
pub use vil_db_sqlx::{SqlxConfig, SqlxPool};

// Re-export bind + query
pub use bind::{VilBind, build_args};
pub use query::VilQuery;

pub mod prelude {
    pub use super::VilEntity;
    pub use super::VilQuery;
    pub use super::pagination::{VilPage, Pagination};
    pub use super::bind::VilBind;
    pub use super::vil_args;
    pub use vil_db_sqlx::{SqlxConfig, SqlxPool};
}
