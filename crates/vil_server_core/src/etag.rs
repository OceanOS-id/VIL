// =============================================================================
// VIL Server — ETag / Conditional Request Middleware
// =============================================================================
//
// Implements HTTP conditional requests (RFC 7232):
//   - Generates ETag from response body hash
//   - Handles If-None-Match → 304 Not Modified
//   - Reduces bandwidth for unchanged resources
//
// Useful for API responses that are cacheable (GET list endpoints, config, etc.)


/// Generate a weak ETag from content bytes.
///
/// Uses FNV-1a hash for speed (not cryptographic — ETags don't need to be).
pub fn generate_etag(content: &[u8]) -> String {
    let hash = fnv1a_hash(content);
    format!("W/\"{:016x}\"", hash)
}

/// Check if the request's If-None-Match header matches the ETag.
pub fn matches_etag(if_none_match: Option<&str>, etag: &str) -> bool {
    if let Some(inm) = if_none_match {
        if inm.trim() == "*" {
            return true;
        }
        // Normalize the etag for comparison
        let etag_normalized = etag.trim_start_matches("W/").trim_matches('"');
        // Handle multiple ETags: If-None-Match: "abc", "def"
        for candidate in inm.split(',') {
            let candidate = candidate.trim().trim_start_matches("W/").trim_matches('"');
            if candidate == etag_normalized {
                return true;
            }
        }
    }
    false
}

/// FNV-1a hash — fast non-cryptographic hash for ETag generation.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// ETag configuration.
#[derive(Debug, Clone)]
pub struct ETagConfig {
    /// Enable ETag generation (default: true)
    pub enabled: bool,
    /// Only generate ETags for responses smaller than this (bytes)
    pub max_body_size: usize,
    /// Content types to generate ETags for
    pub content_types: Vec<String>,
}

impl Default for ETagConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_body_size: 1024 * 1024, // 1MB
            content_types: vec![
                "application/json".to_string(),
                "text/plain".to_string(),
                "text/html".to_string(),
            ],
        }
    }
}
