// =============================================================================
// VIL Server — Multi-Protocol Listener
// =============================================================================
//
// Serve HTTP, gRPC, and WebSocket on the same port using protocol detection.
// Inspects the first bytes of each connection to determine the protocol:
//   - HTTP/1.x: "GET ", "POST", "PUT ", etc.
//   - HTTP/2 (gRPC): PRI * HTTP/2.0 connection preface
//   - WebSocket: HTTP upgrade request
//
// This simplifies deployment — one port for everything.

use serde::Serialize;

/// Protocol detected on a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DetectedProtocol {
    Http1,
    Http2,
    WebSocket,
    Unknown,
}

/// Detect protocol from the first bytes of a connection.
///
/// HTTP/2 preface: "PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n" (24 bytes)
/// HTTP/1.x: starts with method (GET, POST, PUT, DELETE, etc.)
/// WebSocket: HTTP/1.1 with Upgrade: websocket header (detected later)
pub fn detect_protocol(first_bytes: &[u8]) -> DetectedProtocol {
    if first_bytes.len() < 3 {
        return DetectedProtocol::Unknown;
    }

    // HTTP/2 connection preface
    if first_bytes.starts_with(b"PRI ") {
        return DetectedProtocol::Http2;
    }

    // HTTP/1.x methods
    if first_bytes.starts_with(b"GET ")
        || first_bytes.starts_with(b"POST")
        || first_bytes.starts_with(b"PUT ")
        || first_bytes.starts_with(b"DELE")
        || first_bytes.starts_with(b"PATC")
        || first_bytes.starts_with(b"HEAD")
        || first_bytes.starts_with(b"OPTI")
    {
        return DetectedProtocol::Http1;
    }

    DetectedProtocol::Unknown
}

/// Multi-protocol server configuration.
#[derive(Debug, Clone)]
pub struct MultiProtocolConfig {
    /// Listen port (serves HTTP + gRPC + WebSocket)
    pub port: u16,
    /// Enable HTTP/1.1
    pub http1: bool,
    /// Enable HTTP/2 (for gRPC)
    pub http2: bool,
    /// Enable WebSocket upgrade
    pub websocket: bool,
}

impl Default for MultiProtocolConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            http1: true,
            http2: true,
            websocket: true,
        }
    }
}

/// Protocol statistics.
#[derive(Debug, Default, Serialize)]
pub struct ProtocolStats {
    pub http1_connections: u64,
    pub http2_connections: u64,
    pub websocket_connections: u64,
    pub unknown_connections: u64,
}
