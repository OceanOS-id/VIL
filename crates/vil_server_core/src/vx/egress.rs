// =============================================================================
// VX HttpEgress — Response path from endpoint Process back to HTTP client
// =============================================================================
//
// In VX Phase 1, the egress path uses the IngressBridge's oneshot channel
// rather than a separate Process. The endpoint Process directly completes
// the pending request via `bridge.complete(request_id, response)`.
//
// In future phases, this will become a dedicated Process receiving
// ResponseDescriptor from Data Lane and ControlMsg from Control Lane.

use axum::body::Bytes;
use axum::http::StatusCode;

use super::ingress::{IngressBridge, IngressResponse};

/// Egress handle — given to endpoint Process workers to send responses back
/// to the waiting HTTP handler.
///
/// This is a lightweight Clone-able handle that wraps the IngressBridge.
/// Each endpoint Process worker holds one and uses it to complete requests
/// after processing.
#[derive(Clone, Debug)]
pub struct EgressHandle {
    bridge: IngressBridge,
}

impl EgressHandle {
    /// Create a new EgressHandle wrapping the given IngressBridge.
    pub fn new(bridge: IngressBridge) -> Self {
        Self { bridge }
    }

    /// Send a response back to the waiting HTTP handler.
    ///
    /// Returns `true` if the request was still pending and the response was
    /// delivered, `false` if the request had already timed out or was cancelled.
    pub fn respond(
        &self,
        request_id: u64,
        status: StatusCode,
        body: Bytes,
        content_type: &'static str,
    ) -> bool {
        self.bridge.complete(
            request_id,
            IngressResponse {
                status,
                body,
                content_type,
            },
        )
    }

    /// Send an error response back to the waiting HTTP handler.
    ///
    /// Constructs a JSON error body with the given message and status code.
    pub fn respond_error(
        &self,
        request_id: u64,
        status: StatusCode,
        message: &str,
    ) -> bool {
        let error_body = serde_json::json!({
            "error": message,
            "status": status.as_u16(),
        });
        let body = Bytes::from(serde_json::to_vec(&error_body).unwrap_or_default());
        self.respond(request_id, status, body, "application/json")
    }

    /// Get the number of currently pending (in-flight) requests.
    pub fn pending_count(&self) -> usize {
        self.bridge.pending_count()
    }

    /// Get a reference to the underlying IngressBridge.
    pub fn bridge(&self) -> &IngressBridge {
        &self.bridge
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn egress_respond() {
        let bridge = IngressBridge::new();
        let egress = EgressHandle::new(bridge.clone());

        let (req_id, rx) = bridge.register_pending();

        assert!(egress.respond(
            req_id,
            StatusCode::OK,
            Bytes::from_static(b"hello"),
            "text/plain",
        ));

        let resp = rx.await.unwrap();
        assert_eq!(resp.status, StatusCode::OK);
        assert_eq!(resp.body.as_ref(), b"hello");
        assert_eq!(resp.content_type, "text/plain");
    }

    #[tokio::test]
    async fn egress_respond_error() {
        let bridge = IngressBridge::new();
        let egress = EgressHandle::new(bridge.clone());

        let (req_id, rx) = bridge.register_pending();

        assert!(egress.respond_error(
            req_id,
            StatusCode::INTERNAL_SERVER_ERROR,
            "something broke",
        ));

        let resp = rx.await.unwrap();
        assert_eq!(resp.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(resp.content_type, "application/json");

        let body: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
        assert_eq!(body["error"], "something broke");
        assert_eq!(body["status"], 500);
    }

    #[test]
    fn egress_respond_unknown_id() {
        let bridge = IngressBridge::new();
        let egress = EgressHandle::new(bridge);
        assert!(!egress.respond(
            999,
            StatusCode::OK,
            Bytes::new(),
            "text/plain",
        ));
    }

    #[test]
    fn egress_pending_count() {
        let bridge = IngressBridge::new();
        let egress = EgressHandle::new(bridge.clone());

        assert_eq!(egress.pending_count(), 0);
        let (_id, _rx) = bridge.register_pending();
        assert_eq!(egress.pending_count(), 1);
    }

    #[test]
    fn egress_clone_shares_state() {
        let bridge = IngressBridge::new();
        let egress1 = EgressHandle::new(bridge.clone());
        let egress2 = egress1.clone();

        let (_id, _rx) = bridge.register_pending();
        assert_eq!(egress1.pending_count(), 1);
        assert_eq!(egress2.pending_count(), 1);
    }

    #[test]
    fn egress_debug() {
        let bridge = IngressBridge::new();
        let egress = EgressHandle::new(bridge);
        let debug = format!("{:?}", egress);
        assert!(debug.contains("EgressHandle"));
    }
}
