// =============================================================================
// vil_db_mongo::crud — CRUD operations on MongoClient
// =============================================================================
//
// All operations:
//   1. Record `Instant::now()` before the driver call.
//   2. Execute the driver call.
//   3. Emit `db_log!` via `emit_db_log` with timing, op_type, and error_code.
//   4. Return Result<T, MongoFault>.
//
// op_type constants:
//   0 = SELECT (find_one, find_many, count)
//   1 = INSERT (insert_one, insert_many)
//   2 = UPDATE (update_one)
//   3 = DELETE (delete_one)
//
// No println!, tracing::info!, or any non-VIL log call. COMPLIANCE.md §8.
// =============================================================================

use std::time::Instant;

use bson::Document;
use futures_util::TryStreamExt;
use mongodb::options::FindOptions;
use serde::{de::DeserializeOwned, Serialize};

use vil_log::dict::register_str;

use crate::client::{emit_db_log, fault_code_from_mongo_err, MongoClient};
use crate::error::MongoFault;
use crate::types::MongoResult;

// op_type codes
const OP_SELECT: u8 = 0;
const OP_INSERT: u8 = 1;
const OP_UPDATE: u8 = 2;
const OP_DELETE: u8 = 3;

impl MongoClient {
    // =========================================================================
    // find_one
    // =========================================================================

    /// Find a single document matching `filter` in `collection`.
    ///
    /// Emits `db_log!` with `op_type = 0` (SELECT) and `rows_affected = 0|1`.
    pub async fn find_one<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &str,
        filter: Document,
    ) -> MongoResult<Option<T>> {
        let coll_hash = register_str(collection);
        let coll = self.raw_db().collection::<T>(collection);

        let start = Instant::now();
        let result = coll.find_one(filter).await;
        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(doc) => {
                let rows = if doc.is_some() { 1u32 } else { 0u32 };
                emit_db_log(self.db_hash(), collection, OP_SELECT, elapsed_us, rows, 0);
                Ok(doc)
            }
            Err(e) => {
                emit_db_log(self.db_hash(), collection, OP_SELECT, elapsed_us, 0, 1);
                Err(MongoFault::QueryFailed {
                    collection_hash: coll_hash,
                    reason_code: fault_code_from_mongo_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // find_many
    // =========================================================================

    /// Find multiple documents matching `filter` in `collection`.
    ///
    /// Optionally limits results with `limit`. Emits `db_log!` with `op_type = 0`.
    pub async fn find_many<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &str,
        filter: Document,
        limit: Option<i64>,
    ) -> MongoResult<Vec<T>> {
        let coll_hash = register_str(collection);
        let coll = self.raw_db().collection::<T>(collection);

        let mut find_opts = FindOptions::default();
        if let Some(lim) = limit {
            find_opts.limit = Some(lim);
        }

        let start = Instant::now();
        let cursor_result = coll.find(filter).with_options(find_opts).await;
        let elapsed_us = start.elapsed().as_micros() as u32;

        match cursor_result {
            Err(e) => {
                emit_db_log(self.db_hash(), collection, OP_SELECT, elapsed_us, 0, 1);
                Err(MongoFault::QueryFailed {
                    collection_hash: coll_hash,
                    reason_code: fault_code_from_mongo_err(&e),
                })
            }
            Ok(cursor) => {
                let collect_start = Instant::now();
                let docs: Result<Vec<T>, mongodb::error::Error> = cursor.try_collect().await;
                let collect_us = collect_start.elapsed().as_micros() as u32;
                let total_us = elapsed_us.saturating_add(collect_us);

                match docs {
                    Ok(vec) => {
                        let rows = vec.len() as u32;
                        emit_db_log(self.db_hash(), collection, OP_SELECT, total_us, rows, 0);
                        Ok(vec)
                    }
                    Err(e) => {
                        emit_db_log(self.db_hash(), collection, OP_SELECT, total_us, 0, 1);
                        Err(MongoFault::QueryFailed {
                            collection_hash: coll_hash,
                            reason_code: fault_code_from_mongo_err(&e),
                        })
                    }
                }
            }
        }
    }

    // =========================================================================
    // insert_one
    // =========================================================================

    /// Insert a single document into `collection`.
    ///
    /// Returns the string representation of the inserted `_id`.
    /// Emits `db_log!` with `op_type = 1` (INSERT).
    pub async fn insert_one<T: Serialize + Send + Sync>(
        &self,
        collection: &str,
        doc: &T,
    ) -> MongoResult<String> {
        let coll_hash = register_str(collection);
        let coll = self.raw_db().collection::<T>(collection);

        let start = Instant::now();
        let result = coll.insert_one(doc).await;
        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(res) => {
                emit_db_log(self.db_hash(), collection, OP_INSERT, elapsed_us, 1, 0);
                Ok(res.inserted_id.to_string())
            }
            Err(e) => {
                emit_db_log(self.db_hash(), collection, OP_INSERT, elapsed_us, 0, 1);
                Err(MongoFault::InsertFailed {
                    collection_hash: coll_hash,
                    reason_code: fault_code_from_mongo_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // insert_many
    // =========================================================================

    /// Insert multiple documents into `collection`.
    ///
    /// Returns a `Vec<String>` of inserted `_id` values in insertion order.
    /// Emits `db_log!` with `op_type = 1` (INSERT).
    pub async fn insert_many<T: Serialize + Send + Sync>(
        &self,
        collection: &str,
        docs: &[T],
    ) -> MongoResult<Vec<String>> {
        let coll_hash = register_str(collection);
        let coll = self.raw_db().collection::<T>(collection);

        let start = Instant::now();
        let result = coll.insert_many(docs).await;
        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(res) => {
                let rows = res.inserted_ids.len() as u32;
                let ids: Vec<String> = {
                    let mut sorted: Vec<(usize, String)> = res
                        .inserted_ids
                        .into_iter()
                        .map(|(k, v)| (k, v.to_string()))
                        .collect();
                    sorted.sort_by_key(|(k, _)| *k);
                    sorted.into_iter().map(|(_, v)| v).collect()
                };
                emit_db_log(self.db_hash(), collection, OP_INSERT, elapsed_us, rows, 0);
                Ok(ids)
            }
            Err(e) => {
                emit_db_log(self.db_hash(), collection, OP_INSERT, elapsed_us, 0, 1);
                Err(MongoFault::InsertFailed {
                    collection_hash: coll_hash,
                    reason_code: fault_code_from_mongo_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // update_one
    // =========================================================================

    /// Apply `update` to the first document matching `filter` in `collection`.
    ///
    /// Returns the number of documents modified (0 or 1).
    /// Emits `db_log!` with `op_type = 2` (UPDATE).
    pub async fn update_one(
        &self,
        collection: &str,
        filter: Document,
        update: Document,
    ) -> MongoResult<u64> {
        let coll_hash = register_str(collection);
        let coll = self.raw_db().collection::<Document>(collection);

        let start = Instant::now();
        let result = coll.update_one(filter, update).await;
        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(res) => {
                let rows = res.modified_count as u32;
                emit_db_log(self.db_hash(), collection, OP_UPDATE, elapsed_us, rows, 0);
                Ok(res.modified_count)
            }
            Err(e) => {
                emit_db_log(self.db_hash(), collection, OP_UPDATE, elapsed_us, 0, 1);
                Err(MongoFault::UpdateFailed {
                    collection_hash: coll_hash,
                    reason_code: fault_code_from_mongo_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // delete_one
    // =========================================================================

    /// Delete the first document matching `filter` from `collection`.
    ///
    /// Returns the number of documents deleted (0 or 1).
    /// Emits `db_log!` with `op_type = 3` (DELETE).
    pub async fn delete_one(&self, collection: &str, filter: Document) -> MongoResult<u64> {
        let coll_hash = register_str(collection);
        let coll = self.raw_db().collection::<Document>(collection);

        let start = Instant::now();
        let result = coll.delete_one(filter).await;
        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(res) => {
                let rows = res.deleted_count as u32;
                emit_db_log(self.db_hash(), collection, OP_DELETE, elapsed_us, rows, 0);
                Ok(res.deleted_count)
            }
            Err(e) => {
                emit_db_log(self.db_hash(), collection, OP_DELETE, elapsed_us, 0, 1);
                Err(MongoFault::DeleteFailed {
                    collection_hash: coll_hash,
                    reason_code: fault_code_from_mongo_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // count
    // =========================================================================

    /// Count documents in `collection` matching an optional `filter`.
    ///
    /// Pass `None` as filter to count all documents.
    /// Emits `db_log!` with `op_type = 0` (SELECT).
    pub async fn count(&self, collection: &str, filter: Option<Document>) -> MongoResult<u64> {
        let coll_hash = register_str(collection);
        let coll = self.raw_db().collection::<Document>(collection);

        let filter_doc = filter.unwrap_or_default();

        let start = Instant::now();
        let result = coll.count_documents(filter_doc).await;
        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(n) => {
                emit_db_log(
                    self.db_hash(),
                    collection,
                    OP_SELECT,
                    elapsed_us,
                    n as u32,
                    0,
                );
                Ok(n)
            }
            Err(e) => {
                emit_db_log(self.db_hash(), collection, OP_SELECT, elapsed_us, 0, 1);
                Err(MongoFault::QueryFailed {
                    collection_hash: coll_hash,
                    reason_code: fault_code_from_mongo_err(&e),
                })
            }
        }
    }
}
