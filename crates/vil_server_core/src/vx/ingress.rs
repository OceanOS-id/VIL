// =============================================================================
// VX HttpIngress — HTTP boundary Process for VX Tri-Lane
// =============================================================================
//
// Accepts HTTP requests via Axum, writes body to SHM, sends RequestDescriptor
// via Trigger Lane, waits for response, and returns HTTP response.
//
// In Phase 1, the IngressBridge provides a oneshot-based rendezvous between
// the Axum handler (waiting for response) and the endpoint Process worker
// (producing the response). In Phase 2 this will be replaced by direct SHM
// descriptor dispatch at the TCP layer.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::body::Bytes;
use axum::http::StatusCode;
use dashmap::DashMap;
use tokio::sync::oneshot;

/// HTTP ingress configuration for VilApp.
#[derive(Debug, Clone)]
pub struct HttpIngressConfig {
    /// Main HTTP listening port.
    pub port: u16,
    /// Separate metrics/health port (None = same as main port).
    pub metrics_port: Option<u16>,
    /// Enable CORS (permissive by default).
    pub cors: bool,
    /// Maximum request body size in bytes.
    pub max_body_size: usize,
    /// Read timeout in milliseconds (0 = no timeout).
    pub read_timeout_ms: u64,
    /// Write timeout in milliseconds (0 = no timeout).
    pub write_timeout_ms: u64,
}

impl Default for HttpIngressConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            metrics_port: None,
            cors: true,
            max_body_size: 2 * 1024 * 1024, // 2MB
            read_timeout_ms: 30_000,         // 30s
            write_timeout_ms: 30_000,        // 30s
        }
    }
}

impl HttpIngressConfig {
    /// Create a new ingress config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the main HTTP port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the metrics port.
    pub fn metrics_port(mut self, port: u16) -> Self {
        self.metrics_port = Some(port);
        self
    }

    /// Disable CORS.
    pub fn no_cors(mut self) -> Self {
        self.cors = false;
        self
    }

    /// Set maximum request body size in bytes.
    pub fn max_body_size(mut self, size: usize) -> Self {
        self.max_body_size = size;
        self
    }
}

// =============================================================================
// IngressBridge — Oneshot rendezvous between Axum handlers and endpoint workers
// =============================================================================

/// A pending request waiting for a response from an endpoint Process.
pub struct PendingRequest {
    /// Oneshot sender to deliver the response back to the waiting handler.
    pub response_tx: oneshot::Sender<IngressResponse>,
}

/// Response from an endpoint Process back to the ingress handler.
#[derive(Debug)]
pub struct IngressResponse {
    /// HTTP status code for the response.
    pub status: StatusCode,
    /// Response body bytes.
    pub body: Bytes,
    /// Content-Type of the response (e.g., "application/json").
    pub content_type: &'static str,
}

/// The Ingress bridge — shared state between Axum handlers and endpoint Process
/// workers.
///
/// When an HTTP request arrives:
/// 1. Ingress creates a oneshot channel and stores the sender in `pending`
/// 2. Ingress sends descriptor via Tri-Lane Trigger
/// 3. Endpoint Process receives, processes, and sends response back via oneshot
/// 4. Ingress handler `.await`s the oneshot and returns the HTTP response
///
/// This is the Phase 1 mechanism. In Phase 2, the response path will use
/// SHM-backed Data Lane descriptors instead of Tokio oneshot channels.
#[derive(Clone)]
pub struct IngressBridge {
    /// Pending requests: request_id -> oneshot sender
    pub pending: Arc<DashMap<u64, PendingRequest>>,
    /// Monotonic request ID counter
    pub next_request_id: Arc<AtomicU64>,
}

impl IngressBridge {
    /// Create a new IngressBridge with an empty pending map and counter at 1.
    pub fn new() -> Self {
        Self {
            pending: Arc::new(DashMap::new()),
            next_request_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Generate the next unique request ID.
    pub fn next_id(&self) -> u64 {
        self.next_request_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Register a pending request. Returns `(request_id, receiver)`.
    ///
    /// The caller `.await`s the receiver; the endpoint Process sends the
    /// response via `complete()`.
    pub fn register_pending(&self) -> (u64, oneshot::Receiver<IngressResponse>) {
        let id = self.next_id();
        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, PendingRequest { response_tx: tx });
        (id, rx)
    }

    /// Complete a pending request (called by endpoint Process).
    ///
    /// Returns `true` if the request was still pending and the response was
    /// delivered, `false` if the request was already removed (timed out or
    /// cancelled).
    pub fn complete(&self, request_id: u64, response: IngressResponse) -> bool {
        if let Some((_, pending)) = self.pending.remove(&request_id) {
            let _ = pending.response_tx.send(response);
            true
        } else {
            false
        }
    }

    /// Get the number of currently pending (in-flight) requests.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Cancel a pending request (e.g., on timeout).
    ///
    /// Returns `true` if the request was removed.
    pub fn cancel(&self, request_id: u64) -> bool {
        self.pending.remove(&request_id).is_some()
    }
}

impl Default for IngressBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for IngressBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IngressBridge")
            .field("pending_count", &self.pending.len())
            .field(
                "next_request_id",
                &self.next_request_id.load(Ordering::Relaxed),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingress_config_defaults() {
        let cfg = HttpIngressConfig::default();
        assert_eq!(cfg.port, 8080);
        assert!(cfg.cors);
        assert_eq!(cfg.max_body_size, 2 * 1024 * 1024);
    }

    #[test]
    fn ingress_config_builder() {
        let cfg = HttpIngressConfig::new()
            .port(3000)
            .metrics_port(9090)
            .no_cors()
            .max_body_size(1024);

        assert_eq!(cfg.port, 3000);
        assert_eq!(cfg.metrics_port, Some(9090));
        assert!(!cfg.cors);
        assert_eq!(cfg.max_body_size, 1024);
    }

    #[test]
    fn bridge_id_generation() {
        let bridge = IngressBridge::new();
        assert_eq!(bridge.next_id(), 1);
        assert_eq!(bridge.next_id(), 2);
        assert_eq!(bridge.next_id(), 3);
    }

    #[tokio::test]
    async fn bridge_register_and_complete() {
        let bridge = IngressBridge::new();

        // Register a pending request
        let (req_id, rx) = bridge.register_pending();
        assert_eq!(req_id, 1);
        assert_eq!(bridge.pending_count(), 1);

        // Complete the request
        let response = IngressResponse {
            status: StatusCode::OK,
            body: Bytes::from_static(b"{\"ok\":true}"),
            content_type: "application/json",
        };
        assert!(bridge.complete(req_id, response));
        assert_eq!(bridge.pending_count(), 0);

        // Receiver should get the response
        let resp = rx.await.unwrap();
        assert_eq!(resp.status, StatusCode::OK);
        assert_eq!(resp.body.as_ref(), b"{\"ok\":true}");
        assert_eq!(resp.content_type, "application/json");
    }

    #[test]
    fn bridge_complete_unknown_id() {
        let bridge = IngressBridge::new();
        let response = IngressResponse {
            status: StatusCode::OK,
            body: Bytes::new(),
            content_type: "application/json",
        };
        assert!(!bridge.complete(999, response));
    }

    #[test]
    fn bridge_cancel() {
        let bridge = IngressBridge::new();
        let (req_id, _rx) = bridge.register_pending();
        assert_eq!(bridge.pending_count(), 1);
        assert!(bridge.cancel(req_id));
        assert_eq!(bridge.pending_count(), 0);
        // Cancel again should return false
        assert!(!bridge.cancel(req_id));
    }

    #[test]
    fn bridge_clone_shares_state() {
        let bridge1 = IngressBridge::new();
        let bridge2 = bridge1.clone();

        let _ = bridge1.register_pending();
        assert_eq!(bridge2.pending_count(), 1);
    }

    #[test]
    fn bridge_debug() {
        let bridge = IngressBridge::new();
        let debug = format!("{:?}", bridge);
        assert!(debug.contains("IngressBridge"));
        assert!(debug.contains("pending_count"));
    }
}
