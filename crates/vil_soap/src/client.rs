// =============================================================================
// vil_soap::client — SoapClient
// =============================================================================
//
// SOAP/WSDL client with VIL semantic log integration.
//
// - Every call_action() emits db_log! (op_type=4 CALL) with timing.
// - No println!, tracing::info!, or log::info! — COMPLIANCE.md §8.
// - String fields use register_str() hashes — no raw strings on hot path.
// - Uses reqwest with rustls-tls for HTTP transport.
// =============================================================================

use reqwest::Client;
use std::time::Duration;

use vil_log::dict::register_str;
use vil_log::{db_log, types::DbPayload};

use crate::config::SoapConfig;
use crate::envelope::{build_envelope, parse_envelope, ParsedResponse};
use crate::error::SoapFault;

/// SOAP/WSDL client with integrated VIL semantic logging.
///
/// Every `call_action()` automatically emits a `db_log!` entry with:
/// - `db_hash`       — FxHash of the endpoint URL
/// - `table_hash`    — FxHash of the SOAP action name
/// - `duration_us`   — Wall-clock time of the call
/// - `op_type`       — 4 (CALL) for all SOAP RPC calls
/// - `error_code`    — 0 on success, non-zero on fault
///
/// Thread hint: SoapClient is Send+Sync; reqwest manages a connection pool
/// internally. No extra log threads spawned.
pub struct SoapClient {
    http: Client,
    config: SoapConfig,
    /// Cached FxHash of the endpoint URL.
    endpoint_hash: u32,
}

impl SoapClient {
    /// Construct a new `SoapClient` from the given config.
    ///
    /// Creates a shared reqwest Client with the configured timeout.
    pub fn new(config: SoapConfig) -> Result<Self, SoapFault> {
        let endpoint_hash = register_str(&config.endpoint);
        let http = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .use_rustls_tls()
            .build()
            .map_err(|_| SoapFault::ConnectionFailed {
                endpoint_hash,
                reason_code: 1,
            })?;

        Ok(Self {
            http,
            config,
            endpoint_hash,
        })
    }

    /// Call a SOAP action on the configured endpoint.
    ///
    /// Builds the SOAP envelope, sends the HTTP POST, and parses the response.
    /// Emits `db_log!` (op_type=4 CALL) automatically with wall-clock timing.
    ///
    /// # Parameters
    /// - `action`   — SOAP action name (e.g. "GetUser").
    /// - `ns`       — Target namespace URL for the action element.
    /// - `body_xml` — XML content for the SOAP Body element.
    pub async fn call_action(
        &self,
        action: &str,
        ns: &str,
        body_xml: &str,
    ) -> Result<ParsedResponse, SoapFault> {
        let start = std::time::Instant::now();
        let action_hash = register_str(action);

        let envelope = build_envelope(action, ns, body_xml)?;

        let soap_action = format!("\"{}{}\"", ns, action);

        let resp = self
            .http
            .post(&self.config.endpoint)
            .header("Content-Type", "text/xml; charset=utf-8")
            .header("SOAPAction", &soap_action)
            .body(envelope)
            .send()
            .await
            .map_err(|e| {
                let elapsed = start.elapsed();
                let error_code = if e.is_timeout() { 6u8 } else { 1u8 };
                self.emit_db_log(action_hash, elapsed.as_micros() as u32, 0, error_code);
                if e.is_timeout() {
                    SoapFault::Timeout {
                        action_hash,
                        elapsed_ms: elapsed.as_millis() as u32,
                    }
                } else {
                    SoapFault::ConnectionFailed {
                        endpoint_hash: self.endpoint_hash,
                        reason_code: 2,
                    }
                }
            })?;

        let status = resp.status();
        if !status.is_success() {
            let elapsed = start.elapsed();
            self.emit_db_log(action_hash, elapsed.as_micros() as u32, 0, 2);
            return Err(SoapFault::HttpError {
                action_hash,
                status_code: status.as_u16() as u32,
            });
        }

        let xml_body = resp.text().await.map_err(|_| {
            let elapsed = start.elapsed();
            self.emit_db_log(action_hash, elapsed.as_micros() as u32, 0, 5);
            SoapFault::EnvelopeParseFailed {
                action_hash,
                reason_code: 2,
            }
        })?;

        let parsed = parse_envelope(&xml_body, action)?;

        let elapsed = start.elapsed();

        if parsed.is_fault {
            self.emit_db_log(action_hash, elapsed.as_micros() as u32, 0, 3);
            return Err(SoapFault::SoapFaultResponse {
                faultcode_hash: parsed.faultcode_hash,
                faultstring_hash: parsed.faultstring_hash,
            });
        }

        self.emit_db_log(action_hash, elapsed.as_micros() as u32, 1, 0);
        Ok(parsed)
    }

    /// Return the underlying config.
    pub fn config(&self) -> &SoapConfig {
        &self.config
    }

    // -------------------------------------------------------------------------
    // Internal helper — emit db_log! after any call (COMPLIANCE.md §8)
    // -------------------------------------------------------------------------

    fn emit_db_log(&self, action_hash: u32, duration_us: u32, rows_affected: u32, error_code: u8) {
        db_log!(
            Info,
            DbPayload {
                db_hash: self.endpoint_hash,
                table_hash: action_hash,
                query_hash: action_hash,
                duration_us,
                rows_affected,
                op_type: 4, // CALL — RPC-style SOAP
                error_code,
                ..DbPayload::default()
            }
        );
    }
}
