// =============================================================================
// vil_opcua::client — OpcUaClient
// =============================================================================
//
// OPC-UA client wrapper with VIL semantic log integration.
//
// - read_node()  emits db_log! (op_type=0 SELECT) with timing.
// - write_node() emits db_log! (op_type=2 UPDATE) with timing.
// - subscribe()  emits db_log! (op_type=4 CALL) with timing.
// - No println!, tracing::info!, or log::info! — COMPLIANCE.md §8.
// - String fields use register_str() hashes — no raw strings on hot path.
//
// NOTE: The `opcua` crate requires a running OPC-UA server for full operation.
// This implementation provides the API surface with log auto-emit.
// =============================================================================

use std::str::FromStr;
use std::sync::Arc;

use opcua::client::prelude::*;
use opcua::sync::RwLock;

use vil_log::{db_log, types::DbPayload};
use vil_log::dict::register_str;

use crate::config::OpcUaConfig;
use crate::error::OpcUaFault;

/// OPC-UA client with integrated VIL semantic logging.
///
/// Every operation automatically emits a `db_log!` entry with:
/// - `db_hash`      — FxHash of the endpoint URL
/// - `table_hash`   — FxHash of the node ID
/// - `duration_us`  — Wall-clock time of the operation
/// - `op_type`      — 0=read, 2=write, 4=subscribe
/// - `error_code`   — 0 on success, non-zero on fault
///
/// Thread hint: OPC-UA session management spawns internal threads.
/// Add 2 to `LogConfig.threads` for optimal log ring sizing.
pub struct OpcUaClient {
    session: Arc<RwLock<Session>>,
    /// Cached FxHash of the endpoint URL.
    endpoint_hash: u32,
}

impl OpcUaClient {
    /// Connect to an OPC-UA server and return a ready `OpcUaClient`.
    pub fn connect(config: OpcUaConfig) -> Result<Self, OpcUaFault> {
        let endpoint_hash = register_str(&config.endpoint_url);

        let mut client = ClientBuilder::new()
            .application_name(&config.application_name)
            .application_uri("urn:vil_opcua_client")
            .product_uri("urn:vil_opcua")
            .trust_server_certs(true)
            .create_sample_keypair(true)
            .session_retry_limit(3)
            .client()
            .ok_or(OpcUaFault::ConnectionFailed {
                endpoint_hash,
                status_code: 1,
            })?;

        let endpoint = EndpointDescription::from(config.endpoint_url.as_str());

        let session = client
            .connect_to_endpoint(endpoint, IdentityToken::Anonymous)
            .map_err(|e| OpcUaFault::ConnectionFailed {
                endpoint_hash,
                status_code: e.bits(),
            })?;

        Ok(Self { session, endpoint_hash })
    }

    /// Read the value of a single OPC-UA node.
    ///
    /// Emits `db_log!` (op_type=0 SELECT) with timing.
    pub fn read_node(&self, node_id: &str) -> Result<DataValue, OpcUaFault> {
        let start = std::time::Instant::now();
        let node_hash = register_str(node_id);

        let node = NodeId::from_str(node_id).map_err(|e| OpcUaFault::ReadFailed {
            node_hash,
            status_code: e.bits(),
        })?;

        let read_value = ReadValueId {
            node_id: node,
            attribute_id: AttributeId::Value as u32,
            index_range: UAString::null(),
            data_encoding: QualifiedName::null(),
        };

        let result = {
            let session = self.session.read();
            session
                .read(&[read_value], TimestampsToReturn::Both, 0.0)
                .map_err(|e| OpcUaFault::ReadFailed {
                    node_hash,
                    status_code: e.bits(),
                })
                .map(|mut v| v.remove(0))
        };

        let elapsed = start.elapsed();
        let (rows, err_code) = match &result {
            Ok(_)  => (1u32, 0u8),
            Err(f) => (0, f.as_error_code()),
        };

        db_log!(Info, DbPayload {
            db_hash:      self.endpoint_hash,
            table_hash:   node_hash,
            query_hash:   node_hash,
            duration_us:  elapsed.as_micros() as u32,
            rows_affected: rows,
            op_type:      0,   // SELECT / read
            error_code:   err_code,
            ..DbPayload::default()
        });

        result
    }

    /// Write a value to a single OPC-UA node.
    ///
    /// Emits `db_log!` (op_type=2 UPDATE) with timing.
    pub fn write_node(&self, node_id: &str, value: DataValue) -> Result<(), OpcUaFault> {
        let start = std::time::Instant::now();
        let node_hash = register_str(node_id);

        let node = NodeId::from_str(node_id).map_err(|e| OpcUaFault::WriteFailed {
            node_hash,
            status_code: e.bits(),
        })?;

        let write_value = WriteValue {
            node_id: node,
            attribute_id: AttributeId::Value as u32,
            index_range: UAString::null(),
            value,
        };

        let result = {
            let session = self.session.read();
            session
                .write(&[write_value])
                .map_err(|e| OpcUaFault::WriteFailed {
                    node_hash,
                    status_code: e.bits(),
                })
                .map(|_| ())
        };

        let elapsed = start.elapsed();
        let err_code = match &result {
            Ok(_)  => 0u8,
            Err(f) => f.as_error_code(),
        };

        db_log!(Info, DbPayload {
            db_hash:       self.endpoint_hash,
            table_hash:    node_hash,
            query_hash:    node_hash,
            duration_us:   elapsed.as_micros() as u32,
            rows_affected: 1,
            op_type:       2,   // UPDATE / write
            error_code:    err_code,
            ..DbPayload::default()
        });

        result
    }

    /// Subscribe to value changes on a node.
    ///
    /// Returns the subscription ID. Emits `db_log!` (op_type=4 CALL) with timing.
    pub fn subscribe(
        &self,
        node_id: &str,
        publishing_interval_ms: f64,
    ) -> Result<u32, OpcUaFault> {
        let start = std::time::Instant::now();
        let node_hash = register_str(node_id);

        let node = NodeId::from_str(node_id).map_err(|e| OpcUaFault::SubscribeFailed {
            node_hash,
            status_code: e.bits(),
        })?;

        let result = {
            let session = self.session.read();

            let sub_id = session
                .create_subscription(
                    publishing_interval_ms,
                    10,
                    30,
                    0,
                    0,
                    true,
                    DataChangeCallback::new(|_items| {}),
                )
                .map_err(|e| OpcUaFault::SubscribeFailed {
                    node_hash,
                    status_code: e.bits(),
                })?;

            let item = MonitoredItemCreateRequest::new(
                ReadValueId {
                    node_id: node,
                    attribute_id: AttributeId::Value as u32,
                    index_range: UAString::null(),
                    data_encoding: QualifiedName::null(),
                },
                MonitoringMode::Reporting,
                MonitoringParameters::default(),
            );

            session
                .create_monitored_items(sub_id, TimestampsToReturn::Both, &[item])
                .map_err(|e| OpcUaFault::SubscribeFailed {
                    node_hash,
                    status_code: e.bits(),
                })?;

            Ok::<u32, OpcUaFault>(sub_id)
        };

        let elapsed = start.elapsed();
        let err_code = match &result {
            Ok(_)  => 0u8,
            Err(f) => f.as_error_code(),
        };

        db_log!(Info, DbPayload {
            db_hash:       self.endpoint_hash,
            table_hash:    node_hash,
            query_hash:    node_hash,
            duration_us:   elapsed.as_micros() as u32,
            rows_affected: 0,
            op_type:       4,   // CALL — subscribe
            error_code:    err_code,
            ..DbPayload::default()
        });

        result
    }

    /// Return the cached endpoint hash.
    pub fn endpoint_hash(&self) -> u32 {
        self.endpoint_hash
    }
}
