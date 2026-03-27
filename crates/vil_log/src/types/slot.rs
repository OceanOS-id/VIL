// =============================================================================
// vil_log::types::slot — LogSlot (256 bytes total, cache-aligned)
// =============================================================================
//
// Layout:
//   header  : VilLogHeader  = 64 bytes
//   payload : [u8; 192]     = 192 bytes
//   Total   : 256 bytes
// =============================================================================

use super::header::VilLogHeader;

/// A fixed-size 256-byte log slot.
///
/// The `header` holds semantic metadata.
/// The `payload` holds msgpack-encoded or raw structured data.
/// Slots are transported through the SPSC ring without allocation.
#[derive(Clone, Copy)]
#[repr(C, align(64))]
pub struct LogSlot {
    /// Semantic header — 64 bytes.
    pub header: VilLogHeader,
    /// Raw payload bytes — 192 bytes. msgpack or zero-padded.
    pub payload: [u8; 192],
}

impl Default for LogSlot {
    fn default() -> Self {
        // Safety: zeroed bytes are valid for all-u8 payload and integer header fields.
        unsafe { std::mem::zeroed() }
    }
}

impl std::fmt::Debug for LogSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogSlot")
            .field("header", &self.header)
            .field("payload_len", &192usize)
            .finish()
    }
}

// Compile-time size guarantee.
const _: () = {
    assert!(
        std::mem::size_of::<LogSlot>() == 256,
        "LogSlot must be exactly 256 bytes"
    );
};
