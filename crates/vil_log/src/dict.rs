// =============================================================================
// vil_log::dict — DictRegistry
// =============================================================================
//
// Maps (category: u8, hash: u32) → String for reverse lookup.
// Uses fxhash::hash32 for fast, deterministic hashing.
// Thread-safe via std::sync::Mutex (simple, v0.1).
// =============================================================================

use std::collections::HashMap;
use std::sync::Mutex;

use fxhash::hash32;

/// Global dictionary: hash -> string value for reverse lookup.
static DICT: Mutex<Option<HashMap<u32, String>>> = Mutex::new(None);

/// Compute a 32-bit FxHash of a string and register it in the global dict.
///
/// Returns the hash, which can be stored in log headers for compact storage.
pub fn register_str(s: &str) -> u32 {
    let h = hash32(s.as_bytes());
    let mut guard = DICT.lock().unwrap_or_else(|e| e.into_inner());
    let dict = guard.get_or_insert_with(HashMap::new);
    dict.entry(h).or_insert_with(|| s.to_string());
    h
}

/// Look up a string by its hash. Returns None if not registered.
pub fn lookup(hash: u32) -> Option<String> {
    let guard = DICT.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().and_then(|d| d.get(&hash).cloned())
}

/// Number of registered strings.
pub fn dict_size() -> usize {
    let guard = DICT.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().map(|d| d.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_lookup() {
        let h = register_str("hello.world");
        let found = lookup(h);
        assert_eq!(found.as_deref(), Some("hello.world"));
    }

    #[test]
    fn test_idempotent_hash() {
        let h1 = register_str("service.name");
        let h2 = register_str("service.name");
        assert_eq!(h1, h2);
    }
}
