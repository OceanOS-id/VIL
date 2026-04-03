// =============================================================================
// vil_db_dynamodb::ops — DynamoDB operations on DynamoClient
// =============================================================================
//
// All operations:
//   1. Record `Instant::now()` before the SDK call.
//   2. Execute the SDK call.
//   3. Emit `db_log!` via `emit_db_log` with timing, op_type, and error_code.
//   4. Return Result<T, DynamoFault>.
//
// op_type constants:
//   0 = GET    (get_item)
//   1 = PUT    (put_item)
//   2 = UPDATE (update_item)
//   3 = DELETE (delete_item)
//   4 = QUERY  (query)
//   5 = SCAN   (scan)
//
// No println!, tracing::info!, or any non-VIL log call.
// =============================================================================

use std::collections::HashMap;
use std::time::Instant;

use aws_sdk_dynamodb::types::AttributeValue;

use vil_log::dict::register_str;

use crate::client::{emit_db_log, fault_code_from_sdk_err, DynamoClient};
use crate::error::DynamoFault;
use crate::types::DynamoResult;

// op_type codes
const OP_GET: u8 = 0;
const OP_PUT: u8 = 1;
const OP_UPDATE: u8 = 2;
const OP_DELETE: u8 = 3;
const OP_QUERY: u8 = 4;
const OP_SCAN: u8 = 5;

impl DynamoClient {
    // =========================================================================
    // get_item
    // =========================================================================

    /// Fetch a single item from `table` by its primary key `key`.
    ///
    /// Returns `None` if no item matched.
    /// Emits `db_log!` with `op_type = 0` (GET).
    pub async fn get_item(
        &self,
        table: &str,
        key: HashMap<String, AttributeValue>,
    ) -> DynamoResult<Option<HashMap<String, AttributeValue>>> {
        let table_hash = register_str(table);

        let start = Instant::now();
        let result = self
            .raw_client()
            .get_item()
            .table_name(table)
            .set_key(Some(key))
            .send()
            .await;
        let elapsed_ns = start.elapsed().as_nanos() as u64;

        match result {
            Ok(out) => {
                let item = out.item;
                let rows = if item.is_some() { 1u32 } else { 0u32 };
                emit_db_log(
                    self.db_hash(),
                    table,
                    OP_GET,
                    elapsed_ns,
                    rows,
                    0,
                    self.pool_id(),
                );
                Ok(item)
            }
            Err(e) => {
                emit_db_log(
                    self.db_hash(),
                    table,
                    OP_GET,
                    elapsed_ns,
                    0,
                    1,
                    self.pool_id(),
                );
                Err(DynamoFault::GetFailed {
                    table_hash,
                    reason_code: fault_code_from_sdk_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // put_item
    // =========================================================================

    /// Write `item` into `table`, replacing any existing item with the same key.
    ///
    /// Emits `db_log!` with `op_type = 1` (PUT).
    pub async fn put_item(
        &self,
        table: &str,
        item: HashMap<String, AttributeValue>,
    ) -> DynamoResult<()> {
        let table_hash = register_str(table);

        let start = Instant::now();
        let result = self
            .raw_client()
            .put_item()
            .table_name(table)
            .set_item(Some(item))
            .send()
            .await;
        let elapsed_ns = start.elapsed().as_nanos() as u64;

        match result {
            Ok(_) => {
                emit_db_log(
                    self.db_hash(),
                    table,
                    OP_PUT,
                    elapsed_ns,
                    1,
                    0,
                    self.pool_id(),
                );
                Ok(())
            }
            Err(e) => {
                emit_db_log(
                    self.db_hash(),
                    table,
                    OP_PUT,
                    elapsed_ns,
                    0,
                    1,
                    self.pool_id(),
                );
                Err(DynamoFault::PutFailed {
                    table_hash,
                    reason_code: fault_code_from_sdk_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // delete_item
    // =========================================================================

    /// Delete the item identified by `key` from `table`.
    ///
    /// Emits `db_log!` with `op_type = 3` (DELETE).
    pub async fn delete_item(
        &self,
        table: &str,
        key: HashMap<String, AttributeValue>,
    ) -> DynamoResult<()> {
        let table_hash = register_str(table);

        let start = Instant::now();
        let result = self
            .raw_client()
            .delete_item()
            .table_name(table)
            .set_key(Some(key))
            .send()
            .await;
        let elapsed_ns = start.elapsed().as_nanos() as u64;

        match result {
            Ok(_) => {
                emit_db_log(
                    self.db_hash(),
                    table,
                    OP_DELETE,
                    elapsed_ns,
                    1,
                    0,
                    self.pool_id(),
                );
                Ok(())
            }
            Err(e) => {
                emit_db_log(
                    self.db_hash(),
                    table,
                    OP_DELETE,
                    elapsed_ns,
                    0,
                    1,
                    self.pool_id(),
                );
                Err(DynamoFault::DeleteFailed {
                    table_hash,
                    reason_code: fault_code_from_sdk_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // query
    // =========================================================================

    /// Run a `Query` against `table` using the given key-condition expression.
    ///
    /// Returns a `Vec` of attribute maps.
    /// Emits `db_log!` with `op_type = 4` (QUERY).
    pub async fn query(
        &self,
        table: &str,
        key_condition: &str,
        expr_attr_values: HashMap<String, AttributeValue>,
    ) -> DynamoResult<Vec<HashMap<String, AttributeValue>>> {
        let table_hash = register_str(table);

        let start = Instant::now();
        let result = self
            .raw_client()
            .query()
            .table_name(table)
            .key_condition_expression(key_condition)
            .set_expression_attribute_values(Some(expr_attr_values))
            .send()
            .await;
        let elapsed_ns = start.elapsed().as_nanos() as u64;

        match result {
            Ok(out) => {
                let items = out.items.unwrap_or_default();
                let rows = items.len() as u32;
                emit_db_log(
                    self.db_hash(),
                    table,
                    OP_QUERY,
                    elapsed_ns,
                    rows,
                    0,
                    self.pool_id(),
                );
                Ok(items)
            }
            Err(e) => {
                emit_db_log(
                    self.db_hash(),
                    table,
                    OP_QUERY,
                    elapsed_ns,
                    0,
                    1,
                    self.pool_id(),
                );
                Err(DynamoFault::QueryFailed {
                    table_hash,
                    reason_code: fault_code_from_sdk_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // scan
    // =========================================================================

    /// Scan all items in `table`, with an optional filter expression.
    ///
    /// Returns a `Vec` of attribute maps.
    /// Emits `db_log!` with `op_type = 5` (SCAN).
    pub async fn scan(
        &self,
        table: &str,
        filter_expression: Option<&str>,
        expr_attr_values: Option<HashMap<String, AttributeValue>>,
    ) -> DynamoResult<Vec<HashMap<String, AttributeValue>>> {
        let table_hash = register_str(table);

        let mut req = self.raw_client().scan().table_name(table);
        if let Some(fe) = filter_expression {
            req = req.filter_expression(fe);
        }
        if let Some(eav) = expr_attr_values {
            req = req.set_expression_attribute_values(Some(eav));
        }

        let start = Instant::now();
        let result = req.send().await;
        let elapsed_ns = start.elapsed().as_nanos() as u64;

        match result {
            Ok(out) => {
                let items = out.items.unwrap_or_default();
                let rows = items.len() as u32;
                emit_db_log(
                    self.db_hash(),
                    table,
                    OP_SCAN,
                    elapsed_ns,
                    rows,
                    0,
                    self.pool_id(),
                );
                Ok(items)
            }
            Err(e) => {
                emit_db_log(
                    self.db_hash(),
                    table,
                    OP_SCAN,
                    elapsed_ns,
                    0,
                    1,
                    self.pool_id(),
                );
                Err(DynamoFault::ScanFailed {
                    table_hash,
                    reason_code: fault_code_from_sdk_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // update_item
    // =========================================================================

    /// Update an item in `table` using an update expression.
    ///
    /// Emits `db_log!` with `op_type = 2` (UPDATE).
    pub async fn update_item(
        &self,
        table: &str,
        key: HashMap<String, AttributeValue>,
        update_expression: &str,
        expr_attr_values: HashMap<String, AttributeValue>,
    ) -> DynamoResult<()> {
        let table_hash = register_str(table);

        let start = Instant::now();
        let result = self
            .raw_client()
            .update_item()
            .table_name(table)
            .set_key(Some(key))
            .update_expression(update_expression)
            .set_expression_attribute_values(Some(expr_attr_values))
            .send()
            .await;
        let elapsed_ns = start.elapsed().as_nanos() as u64;

        match result {
            Ok(_) => {
                emit_db_log(
                    self.db_hash(),
                    table,
                    OP_UPDATE,
                    elapsed_ns,
                    1,
                    0,
                    self.pool_id(),
                );
                Ok(())
            }
            Err(e) => {
                emit_db_log(
                    self.db_hash(),
                    table,
                    OP_UPDATE,
                    elapsed_ns,
                    0,
                    1,
                    self.pool_id(),
                );
                Err(DynamoFault::UpdateFailed {
                    table_hash,
                    reason_code: fault_code_from_sdk_err(&e),
                })
            }
        }
    }
}
