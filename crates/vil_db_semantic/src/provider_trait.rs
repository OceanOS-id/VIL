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
        &self,
        table: &str,
        key_field: &str,
        key_value: &ToSqlValue,
    ) -> DbResult<Option<Vec<u8>>>;

    /// Find multiple rows with filter.
    async fn find_many(
        &self,
        table: &str,
        filter: &str,
        params: &[ToSqlValue],
    ) -> DbResult<Vec<Vec<u8>>>;

    /// Insert a row. Returns generated ID.
    async fn insert(&self, table: &str, fields: &[&str], values: &[ToSqlValue]) -> DbResult<i64>;

    /// Update a row by primary key. Returns rows affected.
    async fn update(
        &self,
        table: &str,
        key_field: &str,
        key_value: &ToSqlValue,
        fields: &[&str],
        values: &[ToSqlValue],
    ) -> DbResult<u64>;

    /// Delete a row by primary key.
    async fn delete(&self, table: &str, key_field: &str, key_value: &ToSqlValue) -> DbResult<bool>;

    /// Count rows (optional filter).
    async fn count(&self, table: &str, filter: &str, params: &[ToSqlValue]) -> DbResult<u64>;

    /// Raw SQL escape hatch (P2 — non-portable).
    async fn execute_raw(&self, sql: &str, params: &[ToSqlValue]) -> DbResult<u64>;
}

/// Query executor — thin wrapper that adds metrics/caching.
/// Still only 1 vtable call per operation.
#[async_trait::async_trait]
pub trait DbQueryExecutor: Send + Sync {
    async fn execute_find_one(
        &self,
        table: &str,
        key_field: &str,
        key_value: &ToSqlValue,
    ) -> DbResult<Option<Vec<u8>>>;

    async fn execute_find_many(
        &self,
        table: &str,
        filter: &str,
        params: &[ToSqlValue],
    ) -> DbResult<Vec<Vec<u8>>>;

    async fn execute_insert(
        &self,
        table: &str,
        fields: &[&str],
        values: &[ToSqlValue],
    ) -> DbResult<i64>;

    async fn execute_update(
        &self,
        table: &str,
        key_field: &str,
        key_value: &ToSqlValue,
        fields: &[&str],
        values: &[ToSqlValue],
    ) -> DbResult<u64>;

    async fn execute_delete(
        &self,
        table: &str,
        key_field: &str,
        key_value: &ToSqlValue,
    ) -> DbResult<bool>;
}

// =============================================================================
// ProviderExecutor — concrete DbQueryExecutor wrapping a DbProvider.
// Adds per-operation timing and db_log! emission.
// op_type encoding: 0=SELECT 1=INSERT 2=UPDATE 3=DELETE
// =============================================================================

use std::sync::Arc;

/// Concrete executor that wraps any [`DbProvider`] and emits a [`DbPayload`]
/// log event for every query via `db_log!`.
pub struct ProviderExecutor {
    provider: Arc<dyn DbProvider>,
    /// FxHash of the datasource/database name (stored once at construction).
    db_hash: u32,
}

impl ProviderExecutor {
    /// Create a new executor around the given provider.
    pub fn new(provider: Arc<dyn DbProvider>) -> Self {
        let db_hash = {
            use vil_log::dict::register_str;
            register_str(provider.name())
        };
        Self { provider, db_hash }
    }
}

#[async_trait::async_trait]
impl DbQueryExecutor for ProviderExecutor {
    async fn execute_find_one(
        &self,
        table: &str,
        key_field: &str,
        key_value: &ToSqlValue,
    ) -> DbResult<Option<Vec<u8>>> {
        let __db_start = std::time::Instant::now();
        let result = self.provider.find_one(table, key_field, key_value).await;
        let __elapsed = __db_start.elapsed();
        {
            use vil_log::{db_log, dict::register_str, types::DbPayload};
            let __table_hash = register_str(table);
            let __query_hash = register_str(key_field);
            let rows = if matches!(result, Ok(Some(_))) {
                1u32
            } else {
                0u32
            };
            let err: u8 = if result.is_err() { 1 } else { 0 };
            db_log!(
                Info,
                DbPayload {
                    db_hash: self.db_hash,
                    table_hash: __table_hash,
                    query_hash: __query_hash,
                    duration_ns: __elapsed.as_nanos() as u64,
                    rows_affected: rows,
                    op_type: 0, // SELECT
                    prepared: 1,
                    tx_state: 0,
                    error_code: err,
                    ..DbPayload::default()
                }
            );
        }
        result
    }

    async fn execute_find_many(
        &self,
        table: &str,
        filter: &str,
        params: &[ToSqlValue],
    ) -> DbResult<Vec<Vec<u8>>> {
        let __db_start = std::time::Instant::now();
        let result = self.provider.find_many(table, filter, params).await;
        let __elapsed = __db_start.elapsed();
        {
            use vil_log::{db_log, dict::register_str, types::DbPayload};
            let __table_hash = register_str(table);
            let __query_hash = register_str(filter);
            let rows = result.as_ref().map(|v| v.len() as u32).unwrap_or(0);
            let err: u8 = if result.is_err() { 1 } else { 0 };
            db_log!(
                Info,
                DbPayload {
                    db_hash: self.db_hash,
                    table_hash: __table_hash,
                    query_hash: __query_hash,
                    duration_ns: __elapsed.as_nanos() as u64,
                    rows_affected: rows,
                    op_type: 0, // SELECT
                    prepared: 1,
                    tx_state: 0,
                    error_code: err,
                    ..DbPayload::default()
                }
            );
        }
        result
    }

    async fn execute_insert(
        &self,
        table: &str,
        fields: &[&str],
        values: &[ToSqlValue],
    ) -> DbResult<i64> {
        let __db_start = std::time::Instant::now();
        let result = self.provider.insert(table, fields, values).await;
        let __elapsed = __db_start.elapsed();
        {
            use vil_log::{db_log, dict::register_str, types::DbPayload};
            let __table_hash = register_str(table);
            let rows: u32 = if result.is_ok() { 1 } else { 0 };
            let err: u8 = if result.is_err() { 1 } else { 0 };
            db_log!(
                Info,
                DbPayload {
                    db_hash: self.db_hash,
                    table_hash: __table_hash,
                    query_hash: 0,
                    duration_ns: __elapsed.as_nanos() as u64,
                    rows_affected: rows,
                    op_type: 1, // INSERT
                    prepared: 1,
                    tx_state: 0,
                    error_code: err,
                    ..DbPayload::default()
                }
            );
        }
        result
    }

    async fn execute_update(
        &self,
        table: &str,
        key_field: &str,
        key_value: &ToSqlValue,
        fields: &[&str],
        values: &[ToSqlValue],
    ) -> DbResult<u64> {
        let __db_start = std::time::Instant::now();
        let result = self
            .provider
            .update(table, key_field, key_value, fields, values)
            .await;
        let __elapsed = __db_start.elapsed();
        {
            use vil_log::{db_log, dict::register_str, types::DbPayload};
            let __table_hash = register_str(table);
            let __query_hash = register_str(key_field);
            let rows = result.as_ref().copied().unwrap_or(0).min(u32::MAX as u64) as u32;
            let err: u8 = if result.is_err() { 1 } else { 0 };
            db_log!(
                Info,
                DbPayload {
                    db_hash: self.db_hash,
                    table_hash: __table_hash,
                    query_hash: __query_hash,
                    duration_ns: __elapsed.as_nanos() as u64,
                    rows_affected: rows,
                    op_type: 2, // UPDATE
                    prepared: 1,
                    tx_state: 0,
                    error_code: err,
                    ..DbPayload::default()
                }
            );
        }
        result
    }

    async fn execute_delete(
        &self,
        table: &str,
        key_field: &str,
        key_value: &ToSqlValue,
    ) -> DbResult<bool> {
        let __db_start = std::time::Instant::now();
        let result = self.provider.delete(table, key_field, key_value).await;
        let __elapsed = __db_start.elapsed();
        {
            use vil_log::{db_log, dict::register_str, types::DbPayload};
            let __table_hash = register_str(table);
            let __query_hash = register_str(key_field);
            let rows: u32 = if matches!(result, Ok(true)) { 1 } else { 0 };
            let err: u8 = if result.is_err() { 1 } else { 0 };
            db_log!(
                Info,
                DbPayload {
                    db_hash: self.db_hash,
                    table_hash: __table_hash,
                    query_hash: __query_hash,
                    duration_ns: __elapsed.as_nanos() as u64,
                    rows_affected: rows,
                    op_type: 3, // DELETE
                    prepared: 1,
                    tx_state: 0,
                    error_code: err,
                    ..DbPayload::default()
                }
            );
        }
        result
    }
}
