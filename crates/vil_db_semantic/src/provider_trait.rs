// DbProvider trait — the ONLY dynamic dispatch point.
// 1 vtable call per query (~1ns overhead).

use crate::capability::DbCapability;
use crate::error::DbResult;

/// A value that can be passed as a SQL parameter.
/// Avoids generic type complexity — uses JSON as wire format.
#[derive(Debug, Clone)]
pub enum ToSqlValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
}

/// Runtime provider trait — 1 vtable call per operation.
///
/// This is the ONLY point of dynamic dispatch in the entire
/// semantic DB stack. Everything above is compile-time.
#[async_trait::async_trait]
pub trait DbProvider: Send + Sync {
    /// Provider name.
    fn name(&self) -> &str;

    /// Declared capabilities (const in provider, checked at startup).
    fn capabilities(&self) -> DbCapability;

    /// Health check.
    async fn health_check(&self) -> DbResult<()>;

    /// Find one row by primary key.
    async fn find_one(
        &self, table: &str, key_field: &str, key_value: &ToSqlValue,
    ) -> DbResult<Option<Vec<u8>>>;

    /// Find multiple rows with filter.
    async fn find_many(
        &self, table: &str, filter: &str, params: &[ToSqlValue],
    ) -> DbResult<Vec<Vec<u8>>>;

    /// Insert a row. Returns generated ID.
    async fn insert(
        &self, table: &str, fields: &[&str], values: &[ToSqlValue],
    ) -> DbResult<i64>;

    /// Update a row by primary key. Returns rows affected.
    async fn update(
        &self, table: &str, key_field: &str, key_value: &ToSqlValue,
        fields: &[&str], values: &[ToSqlValue],
    ) -> DbResult<u64>;

    /// Delete a row by primary key.
    async fn delete(
        &self, table: &str, key_field: &str, key_value: &ToSqlValue,
    ) -> DbResult<bool>;

    /// Count rows (optional filter).
    async fn count(&self, table: &str, filter: &str, params: &[ToSqlValue]) -> DbResult<u64>;

    /// Raw SQL escape hatch (P2 — non-portable).
    async fn execute_raw(
        &self, sql: &str, params: &[ToSqlValue],
    ) -> DbResult<u64>;
}

/// Query executor — thin wrapper that adds metrics/caching.
/// Still only 1 vtable call per operation.
#[async_trait::async_trait]
pub trait DbQueryExecutor: Send + Sync {
    async fn execute_find_one(
        &self, table: &str, key_field: &str, key_value: &ToSqlValue,
    ) -> DbResult<Option<Vec<u8>>>;

    async fn execute_find_many(
        &self, table: &str, filter: &str, params: &[ToSqlValue],
    ) -> DbResult<Vec<Vec<u8>>>;

    async fn execute_insert(
        &self, table: &str, fields: &[&str], values: &[ToSqlValue],
    ) -> DbResult<i64>;

    async fn execute_update(
        &self, table: &str, key_field: &str, key_value: &ToSqlValue,
        fields: &[&str], values: &[ToSqlValue],
    ) -> DbResult<u64>;

    async fn execute_delete(
        &self, table: &str, key_field: &str, key_value: &ToSqlValue,
    ) -> DbResult<bool>;
}
