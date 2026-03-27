// =============================================================================
// VilSidecarProtocol — Message types for host ↔ sidecar communication
// =============================================================================
//
// Transport: Unix Domain Socket (descriptor only, ~48 bytes per message)
// Data Plane: /dev/shm/vil_sc_{name} (zero-copy via mmap)
//
// Messages flow over UDS as length-prefixed JSON (or MessagePack with feature).
// Actual request/response payloads live in SHM — UDS carries only descriptors.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// SidecarDescriptor — VASI-compliant request/response descriptor
// ---------------------------------------------------------------------------

/// Descriptor pointing to data in a shared SHM region.
///
/// This struct is sent over UDS to tell the other side where to find data
/// in shared memory. The actual payload is NOT sent over the socket.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub struct ShmDescriptor {
    /// Unique request identifier for correlation.
    pub request_id: u64,
    /// SHM region identifier.
    pub region_id: u32,
    /// Padding for alignment.
    pub _pad0: u32,
    /// Byte offset within the SHM region.
    pub offset: u64,
    /// Length of the payload in bytes.
    pub len: u32,
    /// FNV-1a hash of the method name (for fast dispatch).
    pub method_hash: u32,
    /// Execution timeout in milliseconds (0 = no timeout).
    pub timeout_ms: u64,
    /// Flags: bit 0 = batch, bit 1 = priority, bits 2-63 reserved.
    pub flags: u64,
}

impl ShmDescriptor {
    /// Size of the descriptor in bytes (48 bytes, VASI-compliant).
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Create a new descriptor.
    pub fn new(request_id: u64, region_id: u32, offset: u64, len: u32) -> Self {
        Self {
            request_id,
            region_id,
            _pad0: 0,
            offset,
            len,
            method_hash: 0,
            timeout_ms: 0,
            flags: 0,
        }
    }

    /// Set the method hash (FNV-1a of method name).
    pub fn with_method(mut self, method: &str) -> Self {
        self.method_hash = fnv1a_hash(method);
        self
    }

    /// Set the timeout in milliseconds.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// FNV-1a hash for method name dispatch (32-bit).
pub fn fnv1a_hash(s: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in s.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

// ---------------------------------------------------------------------------
// Protocol Messages — exchanged over UDS
// ---------------------------------------------------------------------------

/// All messages in the sidecar protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    /// Sidecar → Host: announce identity and capabilities.
    Handshake(Handshake),
    /// Host → Sidecar: acknowledge handshake.
    HandshakeAck(HandshakeAck),
    /// Host → Sidecar: invoke a handler with data in SHM.
    Invoke(Invoke),
    /// Sidecar → Host: return result with data in SHM.
    Result(InvokeResult),
    /// Host → Sidecar: health check ping.
    Health,
    /// Sidecar → Host: health check response.
    HealthOk(HealthOk),
    /// Host → Sidecar: graceful drain (stop accepting new work).
    Drain,
    /// Sidecar → Host: drain complete (all in-flight done).
    Drained,
    /// Host → Sidecar: shutdown immediately.
    Shutdown,
}

/// Sidecar announces itself to the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handshake {
    /// Sidecar name (must match config).
    pub name: String,
    /// Sidecar version string.
    pub version: String,
    /// List of handler method names this sidecar supports.
    pub methods: Vec<String>,
    /// Capabilities: "async", "batch", "streaming".
    pub capabilities: Vec<String>,
    /// Optional authentication token.
    pub auth_token: Option<String>,
}

/// Host acknowledges the handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeAck {
    /// Whether the handshake was accepted.
    pub accepted: bool,
    /// SHM path for this sidecar's data region.
    pub shm_path: String,
    /// Size of the SHM region in bytes.
    pub shm_size: u64,
    /// Reason if rejected.
    pub reject_reason: Option<String>,
}

/// Host invokes a handler on the sidecar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoke {
    /// Descriptor pointing to request data in SHM.
    pub descriptor: ShmDescriptor,
    /// Method name to invoke.
    pub method: String,
}

/// Sidecar returns the result to the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeResult {
    /// Request ID (for correlation with the original Invoke).
    pub request_id: u64,
    /// Status of the invocation.
    pub status: InvokeStatus,
    /// Descriptor pointing to response data in SHM (if Ok).
    pub descriptor: Option<ShmDescriptor>,
    /// Error message (if Error).
    pub error: Option<String>,
}

/// Status of a sidecar invocation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvokeStatus {
    Ok,
    Error,
    Timeout,
    MethodNotFound,
}

/// Health check response from sidecar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthOk {
    /// Number of requests currently in flight.
    pub in_flight: u64,
    /// Total requests processed since startup.
    pub total_processed: u64,
    /// Total errors since startup.
    pub total_errors: u64,
    /// Uptime in seconds.
    pub uptime_secs: u64,
}

// ---------------------------------------------------------------------------
// Wire format — length-prefixed JSON over UDS
// ---------------------------------------------------------------------------

/// Encode a message to bytes (4-byte length prefix + JSON payload).
pub fn encode_message(msg: &Message) -> std::result::Result<Vec<u8>, serde_json::Error> {
    let json = serde_json::to_vec(msg)?;
    let len = json.len() as u32;
    let mut buf = Vec::with_capacity(4 + json.len());
    buf.extend_from_slice(&len.to_le_bytes());
    buf.extend_from_slice(&json);
    Ok(buf)
}

/// Decode a message from a complete frame (JSON payload without length prefix).
pub fn decode_message(data: &[u8]) -> std::result::Result<Message, serde_json::Error> {
    serde_json::from_slice(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_size() {
        assert_eq!(ShmDescriptor::SIZE, 48);
    }

    #[test]
    fn test_fnv1a_hash_deterministic() {
        let h1 = fnv1a_hash("fraud_check");
        let h2 = fnv1a_hash("fraud_check");
        assert_eq!(h1, h2);
        assert_ne!(h1, fnv1a_hash("validate_order"));
    }

    #[test]
    fn test_message_roundtrip() {
        let msg = Message::Handshake(Handshake {
            name: "fraud-checker".into(),
            version: "1.0.0".into(),
            methods: vec!["fraud_check".into(), "batch_score".into()],
            capabilities: vec!["async".into()],
            auth_token: None,
        });

        let encoded = encode_message(&msg).unwrap();
        // Skip 4-byte length prefix
        let decoded = decode_message(&encoded[4..]).unwrap();

        match decoded {
            Message::Handshake(h) => {
                assert_eq!(h.name, "fraud-checker");
                assert_eq!(h.methods.len(), 2);
            }
            _ => panic!("expected Handshake"),
        }
    }

    #[test]
    fn test_invoke_result_roundtrip() {
        let msg = Message::Result(InvokeResult {
            request_id: 42,
            status: InvokeStatus::Ok,
            descriptor: Some(ShmDescriptor::new(42, 1, 0, 256)),
            error: None,
        });

        let encoded = encode_message(&msg).unwrap();
        let decoded = decode_message(&encoded[4..]).unwrap();

        match decoded {
            Message::Result(r) => {
                assert_eq!(r.request_id, 42);
                assert_eq!(r.status, InvokeStatus::Ok);
                assert!(r.descriptor.is_some());
            }
            _ => panic!("expected Result"),
        }
    }

    #[test]
    fn test_health_roundtrip() {
        let msg = Message::Health;
        let encoded = encode_message(&msg).unwrap();
        let decoded = decode_message(&encoded[4..]).unwrap();
        assert!(matches!(decoded, Message::Health));
    }

    #[test]
    fn test_descriptor_with_method() {
        let desc = ShmDescriptor::new(1, 0, 0, 128)
            .with_method("fraud_check")
            .with_timeout(5000);

        assert_eq!(desc.method_hash, fnv1a_hash("fraud_check"));
        assert_eq!(desc.timeout_ms, 5000);
    }
}
