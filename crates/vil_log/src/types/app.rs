// =============================================================================
// vil_log::types::app — AppPayload
// =============================================================================
//
// General application log payload. Carries a code string hash + msgpack KV.
// The msgpack-encoded key-value pairs are stored inline in `kv_bytes`.
// =============================================================================

/// General application event payload. Fits in 192 bytes.
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct AppPayload {
    /// FxHash of the event code string (e.g. "user.login").
    pub code_hash: u32,
    /// Length of valid data in `kv_bytes`.
    pub kv_len: u16,
    /// Padding.
    pub _pad: [u8; 2],
    /// Inline msgpack-encoded key-value pairs.
    pub kv_bytes: [u8; 184],
}

impl Default for AppPayload {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

const _: () = {
    assert!(
        std::mem::size_of::<AppPayload>() <= 192,
        "AppPayload must fit within 192 bytes"
    );
};
