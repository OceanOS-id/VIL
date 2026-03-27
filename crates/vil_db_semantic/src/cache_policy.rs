// 8 byte enum — stack-allocated hint for SHM cache.

/// Cache policy hint for query results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CachePolicy {
    /// No caching.
    None,
    /// Cache with TTL (seconds).
    Ttl(u32),
    /// Invalidate on any write to this entity's table.
    InvalidateOnWrite,
    /// Share cached result across all mesh services via SHM.
    SharedAcrossServices,
}

impl Default for CachePolicy { fn default() -> Self { Self::None } }
