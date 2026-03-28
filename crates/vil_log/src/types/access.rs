// =============================================================================
// vil_log::types::access — AccessPayload
// =============================================================================
//
// HTTP access log fields. Must fit within 192 bytes.
// Layout: all fixed-size fields, no heap allocation.
// =============================================================================

/// HTTP/RPC access log payload. Fits in 192 bytes.
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct AccessPayload {
    /// HTTP method code: 0=GET 1=POST 2=PUT 3=DELETE 4=PATCH 5=HEAD 6=OPTIONS
    pub method: u8,
    /// HTTP status code (e.g. 200, 404, 500).
    pub status_code: u16,
    /// Protocol version: 0=HTTP/1.1 1=HTTP/2 2=HTTP/3 3=gRPC
    pub protocol: u8,
    /// Request duration in microseconds.
    pub duration_us: u32,
    /// Request body size in bytes.
    pub request_bytes: u32,
    /// Response body size in bytes.
    pub response_bytes: u32,
    /// Client IPv4 address (packed u32, big-endian).
    pub client_ip: u32,
    /// Server port.
    pub server_port: u16,
    /// FxHash of the route/path template.
    pub route_hash: u32,
    /// FxHash of the user agent string.
    pub user_agent_hash: u32,
    /// FxHash of the request path (full).
    pub path_hash: u32,
    /// Session/correlation token (first 8 bytes).
    pub session_id: u64,
    /// Whether the request was authenticated.
    pub authenticated: u8,
    /// Cache hit indicator: 0=miss 1=hit 2=stale
    pub cache_status: u8,
    /// Padding to 64 bytes.
    pub _pad: [u8; 18],
}

impl Default for AccessPayload {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

const _: () = {
    assert!(
        std::mem::size_of::<AccessPayload>() <= 192,
        "AccessPayload must fit within 192 bytes"
    );
};
