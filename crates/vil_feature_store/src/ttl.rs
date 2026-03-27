// =============================================================================
// D16 — TTL-Based Eviction
// =============================================================================

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current timestamp in milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_millis() as u64
}

/// Checks whether an entry with the given `created_at` and `ttl_ms` has expired.
///
/// Returns `true` if the entry has expired, `false` otherwise.
/// If `ttl_ms` is `None`, the entry never expires.
pub fn is_expired(created_at: u64, ttl_ms: Option<u64>) -> bool {
    match ttl_ms {
        None => false,
        Some(ttl) => {
            let now = now_ms();
            now > created_at + ttl
        }
    }
}
