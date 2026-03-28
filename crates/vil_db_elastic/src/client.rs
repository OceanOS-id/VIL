// =============================================================================
// vil_db_elastic::client — ElasticClient
// =============================================================================
//
// Elasticsearch / OpenSearch client wrapping the `elasticsearch` crate.
//
// Every public operation:
//   1. Records `Instant::now()` before the call.
//   2. Executes the Elasticsearch API call.
//   3. Emits `db_log!` with timing on both success and error paths.
//
// No `println!`, `tracing::info!`, or `eprintln!` are used.
// All string fields in log payloads use `register_str()` hashes.
// =============================================================================

use std::time::Instant;

use elasticsearch::auth::Credentials;
use elasticsearch::http::transport::{SingleNodeConnectionPool, TransportBuilder};
use elasticsearch::http::Url;
use elasticsearch::{
    indices::IndicesCreateParts, BulkOperation, BulkParts, DeleteParts, Elasticsearch, GetParts,
    IndexParts, SearchParts,
};
use serde_json::Value;

use vil_log::dict::register_str;
use vil_log::{db_log, types::DbPayload};

use crate::config::ElasticConfig;
use crate::error::ElasticFault;

// op_type constants
const OP_SELECT: u8 = 0; // SELECT — search, get
const OP_INSERT: u8 = 1; // INSERT — index, bulk
const OP_DELETE: u8 = 3; // DELETE — delete
const OP_DDL: u8 = 5; // DDL    — create_index

// =============================================================================
// Result types
// =============================================================================

/// Result returned by a successful `index` (insert/upsert) call.
#[derive(Debug, Clone)]
pub struct IndexResult {
    /// The document ID assigned or used.
    pub id: String,
    /// The index the document was stored in.
    pub index: String,
    /// The result action: "created", "updated", "noop", etc.
    pub result: String,
    /// The document version after the operation.
    pub version: Option<i64>,
}

/// Result returned by a successful `search` call.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Total number of matching documents.
    pub total: u64,
    /// The raw `hits.hits` array from the Elasticsearch response.
    pub hits: Vec<Value>,
}

// =============================================================================
// ElasticClient
// =============================================================================

/// Elasticsearch / OpenSearch client.
///
/// Build one via [`ElasticClient::new`] with an [`ElasticConfig`], then use
/// the async methods to interact with Elasticsearch.
///
/// Every method auto-emits a `db_log!` entry with operation timing.
pub struct ElasticClient {
    inner: Elasticsearch,
    /// FxHash of the URL for log payloads.
    config_hash: u32,
}

impl ElasticClient {
    /// Create a new `ElasticClient` from the provided configuration.
    pub fn new(config: ElasticConfig) -> Result<Self, ElasticFault> {
        let config_hash = register_str(&config.url);

        let url = Url::parse(&config.url).map_err(|_| ElasticFault::ConnectionFailed {
            url_hash: config_hash,
            reason_code: 1,
        })?;

        let conn_pool = SingleNodeConnectionPool::new(url);
        let mut builder = TransportBuilder::new(conn_pool);

        if let (Some(user), Some(pass)) = (config.username, config.password) {
            builder = builder.auth(Credentials::Basic(user, pass));
        }

        let transport = builder
            .build()
            .map_err(|e| ElasticFault::ConnectionFailed {
                url_hash: register_str(&e.to_string()),
                reason_code: 2,
            })?;

        let inner = Elasticsearch::new(transport);

        Ok(Self { inner, config_hash })
    }

    // =========================================================================
    // index (insert / upsert)
    // =========================================================================

    /// Index a document (`body`) with the given `id` into `index_name`.
    ///
    /// If a document with `id` already exists it will be replaced.
    pub async fn index(
        &self,
        index_name: &str,
        id: &str,
        body: Value,
    ) -> Result<IndexResult, ElasticFault> {
        let start = Instant::now();
        let index_hash = register_str(index_name);
        let id_hash = register_str(id);

        let result = self
            .inner
            .index(IndexParts::IndexId(index_name, id))
            .body(body)
            .send()
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(response) => {
                let status = response.status_code().as_u16();
                if status >= 400 {
                    let fault = classify_status(status, index_hash, id_hash);

                    db_log!(
                        Info,
                        DbPayload {
                            db_hash: self.config_hash,
                            table_hash: index_hash,
                            query_hash: id_hash,
                            duration_us: elapsed.as_micros() as u32,
                            rows_affected: 0,
                            op_type: OP_INSERT,
                            error_code: 1,
                            ..Default::default()
                        }
                    );

                    return Err(fault);
                }

                let json: Value = response.json().await.map_err(|e| ElasticFault::Unknown {
                    message_hash: register_str(&e.to_string()),
                })?;

                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.config_hash,
                        table_hash: index_hash,
                        query_hash: id_hash,
                        duration_us: elapsed.as_micros() as u32,
                        rows_affected: 1,
                        op_type: OP_INSERT,
                        error_code: 0,
                        ..Default::default()
                    }
                );

                Ok(IndexResult {
                    id: json["_id"].as_str().unwrap_or(id).to_owned(),
                    index: json["_index"].as_str().unwrap_or(index_name).to_owned(),
                    result: json["result"].as_str().unwrap_or("unknown").to_owned(),
                    version: json["_version"].as_i64(),
                })
            }
            Err(e) => {
                let fault = ElasticFault::IndexFailed {
                    index_hash,
                    id_hash,
                };

                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.config_hash,
                        table_hash: index_hash,
                        query_hash: register_str(&e.to_string()),
                        duration_us: elapsed.as_micros() as u32,
                        rows_affected: 0,
                        op_type: OP_INSERT,
                        error_code: 1,
                        ..Default::default()
                    }
                );

                Err(fault)
            }
        }
    }

    // =========================================================================
    // search
    // =========================================================================

    /// Execute a search query against `index_name`.
    ///
    /// `query` should be a valid Elasticsearch query DSL `Value`, e.g.
    /// `serde_json::json!({"query": {"match_all": {}}})`.
    pub async fn search(
        &self,
        index_name: &str,
        query: Value,
    ) -> Result<SearchResult, ElasticFault> {
        let start = Instant::now();
        let index_hash = register_str(index_name);
        let query_hash = register_str(&query.to_string());

        let result = self
            .inner
            .search(SearchParts::Index(&[index_name]))
            .body(query)
            .send()
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(response) => {
                let status = response.status_code().as_u16();
                if status >= 400 {
                    let fault = classify_status(status, index_hash, query_hash);

                    db_log!(
                        Info,
                        DbPayload {
                            db_hash: self.config_hash,
                            table_hash: index_hash,
                            query_hash,
                            duration_us: elapsed.as_micros() as u32,
                            rows_affected: 0,
                            op_type: OP_SELECT,
                            error_code: 1,
                            ..Default::default()
                        }
                    );

                    return Err(fault);
                }

                let json: Value = response.json().await.map_err(|e| ElasticFault::Unknown {
                    message_hash: register_str(&e.to_string()),
                })?;

                let total = json["hits"]["total"]["value"].as_u64().unwrap_or(0);
                let hits = json["hits"]["hits"].as_array().cloned().unwrap_or_default();

                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.config_hash,
                        table_hash: index_hash,
                        query_hash,
                        duration_us: elapsed.as_micros() as u32,
                        rows_affected: hits.len() as u32,
                        op_type: OP_SELECT,
                        error_code: 0,
                        ..Default::default()
                    }
                );

                Ok(SearchResult { total, hits })
            }
            Err(e) => {
                let fault = ElasticFault::SearchFailed {
                    index_hash,
                    query_hash,
                };

                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.config_hash,
                        table_hash: index_hash,
                        query_hash: register_str(&e.to_string()),
                        duration_us: elapsed.as_micros() as u32,
                        rows_affected: 0,
                        op_type: OP_SELECT,
                        error_code: 1,
                        ..Default::default()
                    }
                );

                Err(fault)
            }
        }
    }

    // =========================================================================
    // get
    // =========================================================================

    /// Retrieve a document by `id` from `index_name`.
    ///
    /// Returns `ElasticFault::NotFound` if the document does not exist.
    pub async fn get(&self, index_name: &str, id: &str) -> Result<Value, ElasticFault> {
        let start = Instant::now();
        let index_hash = register_str(index_name);
        let id_hash = register_str(id);

        let result = self
            .inner
            .get(GetParts::IndexId(index_name, id))
            .send()
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(response) => {
                let status = response.status_code().as_u16();
                if status == 404 {
                    db_log!(
                        Info,
                        DbPayload {
                            db_hash: self.config_hash,
                            table_hash: index_hash,
                            query_hash: id_hash,
                            duration_us: elapsed.as_micros() as u32,
                            rows_affected: 0,
                            op_type: OP_SELECT,
                            error_code: 1,
                            ..Default::default()
                        }
                    );

                    return Err(ElasticFault::NotFound {
                        index_hash,
                        id_hash,
                    });
                }

                if status >= 400 {
                    let fault = classify_status(status, index_hash, id_hash);

                    db_log!(
                        Info,
                        DbPayload {
                            db_hash: self.config_hash,
                            table_hash: index_hash,
                            query_hash: id_hash,
                            duration_us: elapsed.as_micros() as u32,
                            rows_affected: 0,
                            op_type: OP_SELECT,
                            error_code: 1,
                            ..Default::default()
                        }
                    );

                    return Err(fault);
                }

                let json: Value = response.json().await.map_err(|e| ElasticFault::Unknown {
                    message_hash: register_str(&e.to_string()),
                })?;

                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.config_hash,
                        table_hash: index_hash,
                        query_hash: id_hash,
                        duration_us: elapsed.as_micros() as u32,
                        rows_affected: 1,
                        op_type: OP_SELECT,
                        error_code: 0,
                        ..Default::default()
                    }
                );

                Ok(json["_source"].clone())
            }
            Err(e) => {
                let fault = ElasticFault::Unknown {
                    message_hash: register_str(&e.to_string()),
                };

                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.config_hash,
                        table_hash: index_hash,
                        query_hash: id_hash,
                        duration_us: elapsed.as_micros() as u32,
                        rows_affected: 0,
                        op_type: OP_SELECT,
                        error_code: 1,
                        ..Default::default()
                    }
                );

                Err(fault)
            }
        }
    }

    // =========================================================================
    // delete
    // =========================================================================

    /// Delete the document with `id` from `index_name`.
    pub async fn delete(&self, index_name: &str, id: &str) -> Result<(), ElasticFault> {
        let start = Instant::now();
        let index_hash = register_str(index_name);
        let id_hash = register_str(id);

        let result = self
            .inner
            .delete(DeleteParts::IndexId(index_name, id))
            .send()
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(response) => {
                let status = response.status_code().as_u16();
                if status >= 400 {
                    let fault = classify_status(status, index_hash, id_hash);

                    db_log!(
                        Info,
                        DbPayload {
                            db_hash: self.config_hash,
                            table_hash: index_hash,
                            query_hash: id_hash,
                            duration_us: elapsed.as_micros() as u32,
                            rows_affected: 0,
                            op_type: OP_DELETE,
                            error_code: 1,
                            ..Default::default()
                        }
                    );

                    return Err(fault);
                }

                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.config_hash,
                        table_hash: index_hash,
                        query_hash: id_hash,
                        duration_us: elapsed.as_micros() as u32,
                        rows_affected: 1,
                        op_type: OP_DELETE,
                        error_code: 0,
                        ..Default::default()
                    }
                );

                Ok(())
            }
            Err(e) => {
                let fault = ElasticFault::Unknown {
                    message_hash: register_str(&e.to_string()),
                };

                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.config_hash,
                        table_hash: index_hash,
                        query_hash: id_hash,
                        duration_us: elapsed.as_micros() as u32,
                        rows_affected: 0,
                        op_type: OP_DELETE,
                        error_code: 1,
                        ..Default::default()
                    }
                );

                Err(fault)
            }
        }
    }

    // =========================================================================
    // bulk
    // =========================================================================

    /// Execute a bulk index operation for `docs` into `index_name`.
    ///
    /// Each element in `docs` is a `(id, body)` pair. The operation uses
    /// bulk index semantics (insert or replace).
    ///
    /// Returns the number of successfully indexed documents.
    pub async fn bulk(
        &self,
        index_name: &str,
        docs: Vec<(String, Value)>,
    ) -> Result<u32, ElasticFault> {
        let start = Instant::now();
        let index_hash = register_str(index_name);

        let operations: Vec<BulkOperation<Value>> = docs
            .into_iter()
            .map(|(id, body)| BulkOperation::index(body).id(id).into())
            .collect();

        let result = self
            .inner
            .bulk(BulkParts::Index(index_name))
            .body(operations)
            .send()
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(response) => {
                let status = response.status_code().as_u16();
                if status >= 400 {
                    let fault = classify_status(status, index_hash, 0);

                    db_log!(
                        Info,
                        DbPayload {
                            db_hash: self.config_hash,
                            table_hash: index_hash,
                            query_hash: register_str("bulk"),
                            duration_us: elapsed.as_micros() as u32,
                            rows_affected: 0,
                            op_type: OP_INSERT,
                            error_code: 1,
                            ..Default::default()
                        }
                    );

                    return Err(fault);
                }

                let json: Value = response.json().await.map_err(|e| ElasticFault::Unknown {
                    message_hash: register_str(&e.to_string()),
                })?;

                // Count errors in the bulk response
                let items = json["items"].as_array().map(|a| a.len()).unwrap_or(0);
                let errors = json["errors"].as_bool().unwrap_or(false);

                if errors {
                    // Count failed items
                    let failed = json["items"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter(|item| {
                                    item["index"]["error"].is_object()
                                        || item["create"]["error"].is_object()
                                })
                                .count() as u32
                        })
                        .unwrap_or(0);

                    db_log!(
                        Info,
                        DbPayload {
                            db_hash: self.config_hash,
                            table_hash: index_hash,
                            query_hash: register_str("bulk"),
                            duration_us: elapsed.as_micros() as u32,
                            rows_affected: items as u32 - failed,
                            op_type: OP_INSERT,
                            error_code: if failed > 0 { 1 } else { 0 },
                            ..Default::default()
                        }
                    );

                    if failed > 0 {
                        return Err(ElasticFault::BulkFailed {
                            index_hash,
                            failed_count: failed,
                        });
                    }
                }

                let succeeded = items as u32;

                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.config_hash,
                        table_hash: index_hash,
                        query_hash: register_str("bulk"),
                        duration_us: elapsed.as_micros() as u32,
                        rows_affected: succeeded,
                        op_type: OP_INSERT,
                        error_code: 0,
                        ..Default::default()
                    }
                );

                Ok(succeeded)
            }
            Err(e) => {
                let fault = ElasticFault::Unknown {
                    message_hash: register_str(&e.to_string()),
                };

                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.config_hash,
                        table_hash: index_hash,
                        query_hash: register_str("bulk"),
                        duration_us: elapsed.as_micros() as u32,
                        rows_affected: 0,
                        op_type: OP_INSERT,
                        error_code: 1,
                        ..Default::default()
                    }
                );

                Err(fault)
            }
        }
    }

    // =========================================================================
    // create_index
    // =========================================================================

    /// Create an index named `index_name` with optional `settings` JSON.
    ///
    /// If `settings` is `None`, the index is created with default settings.
    pub async fn create_index(
        &self,
        index_name: &str,
        settings: Option<Value>,
    ) -> Result<(), ElasticFault> {
        let start = Instant::now();
        let index_hash = register_str(index_name);

        let body = settings.unwrap_or(serde_json::json!({}));

        let result = self
            .inner
            .indices()
            .create(IndicesCreateParts::Index(index_name))
            .body(body)
            .send()
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(response) => {
                let status = response.status_code().as_u16();
                if status >= 400 {
                    let fault = classify_status(status, index_hash, 0);

                    db_log!(
                        Info,
                        DbPayload {
                            db_hash: self.config_hash,
                            table_hash: index_hash,
                            query_hash: register_str("create_index"),
                            duration_us: elapsed.as_micros() as u32,
                            rows_affected: 0,
                            op_type: OP_DDL,
                            error_code: 1,
                            ..Default::default()
                        }
                    );

                    return Err(fault);
                }

                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.config_hash,
                        table_hash: index_hash,
                        query_hash: register_str("create_index"),
                        duration_us: elapsed.as_micros() as u32,
                        rows_affected: 1,
                        op_type: OP_DDL,
                        error_code: 0,
                        ..Default::default()
                    }
                );

                Ok(())
            }
            Err(e) => {
                let fault = ElasticFault::Unknown {
                    message_hash: register_str(&e.to_string()),
                };

                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.config_hash,
                        table_hash: index_hash,
                        query_hash: register_str("create_index"),
                        duration_us: elapsed.as_micros() as u32,
                        rows_affected: 0,
                        op_type: OP_DDL,
                        error_code: 1,
                        ..Default::default()
                    }
                );

                Err(fault)
            }
        }
    }
}

// =============================================================================
// Error classification helpers
// =============================================================================

fn classify_status(status: u16, index_hash: u32, id_hash: u32) -> ElasticFault {
    match status {
        401 | 403 => ElasticFault::AccessDenied { index_hash },
        404 => ElasticFault::NotFound {
            index_hash,
            id_hash,
        },
        _ => ElasticFault::Unknown {
            message_hash: register_str("elastic_http_error"),
        },
    }
}
