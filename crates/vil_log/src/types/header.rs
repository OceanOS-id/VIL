// =============================================================================
// vil_log::types::header — VilLogHeader (64 bytes, cache-aligned)
// =============================================================================
//
// Exactly 64 bytes. Verified via static assertion below.
// Layout (all fields, no padding):
//   event_id      : u128  = 16 bytes   offset 0
//   trace_id      : u64   =  8 bytes   offset 16
//   tenant_id     : u64   =  8 bytes   offset 24
//   process_id    : u64   =  8 bytes   offset 32
//   timestamp_ns  : u64   =  8 bytes   offset 40
//   level         : u8    =  1 byte    offset 48
//   category      : u8    =  1 byte    offset 49
//   subcategory   : u8    =  1 byte    offset 50
//   version       : u8    =  1 byte    offset 51
//   service_hash  : u32   =  4 bytes   offset 52
//   handler_hash  : u32   =  4 bytes   offset 56
//   node_hash     : u32   =  4 bytes   offset 60
//   Total: 64 bytes
// =============================================================================

/// Structured log header. 64 bytes, cache-line aligned.
///
/// Placed at the start of every `LogSlot`. Zero-copy friendly.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C, align(64))]
pub struct VilLogHeader {
    /// Unique event identifier (UUID-level uniqueness).
    pub event_id: u128,
    /// Distributed trace ID for correlation.
    pub trace_id: u64,
    /// Tenant identifier for multi-tenant deployments.
    pub tenant_id: u64,
    /// OS process ID.
    pub process_id: u64,
    /// Event timestamp in nanoseconds since Unix epoch.
    pub timestamp_ns: u64,
    /// Log severity level (see `LogLevel`).
    pub level: u8,
    /// Top-level category (see `LogCategory`).
    pub category: u8,
    /// Sub-category, category-specific meaning.
    pub subcategory: u8,
    /// Schema version for forward-compat.
    pub version: u8,
    /// FxHash of the service name string.
    pub service_hash: u32,
    /// FxHash of the handler/function name.
    pub handler_hash: u32,
    /// FxHash of the node/host identifier.
    pub node_hash: u32,
}

// Compile-time size guarantee.
const _: () = {
    assert!(
        std::mem::size_of::<VilLogHeader>() == 64,
        "VilLogHeader must be exactly 64 bytes"
    );
};
