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

// =============================================================================
// Persistence — save/load dictionary to/from JSON file
// =============================================================================

/// Save the current dictionary to a JSON file.
/// Format: `{ "hash_decimal": "original_string", ... }`
pub fn save_to_file(path: &std::path::Path) -> std::io::Result<()> {
    let guard = DICT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(dict) = guard.as_ref() {
        let map: std::collections::BTreeMap<String, &String> = dict
            .iter()
            .map(|(k, v)| (format!("{}", k), v))
            .collect();
        let json = serde_json::to_string_pretty(&map)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)?;
    }
    Ok(())
}

/// Load dictionary from a JSON file (merges with existing entries).
pub fn load_from_file(path: &std::path::Path) -> std::io::Result<usize> {
    let json = std::fs::read_to_string(path)?;
    let map: std::collections::BTreeMap<String, String> = serde_json::from_str(&json)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut guard = DICT.lock().unwrap_or_else(|e| e.into_inner());
    let dict = guard.get_or_insert_with(HashMap::new);
    let mut loaded = 0;
    for (hash_str, value) in map {
        if let Ok(hash) = hash_str.parse::<u32>() {
            dict.entry(hash).or_insert_with(|| {
                loaded += 1;
                value
            });
        }
    }
    Ok(loaded)
}

/// Export the full dictionary as a HashMap (for external use).
pub fn export_all() -> HashMap<u32, String> {
    let guard = DICT.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().cloned().unwrap_or_default()
}

// =============================================================================
// Resolve helpers — decode known enum values to human strings
// =============================================================================

/// Resolve op_type to human-readable string.
pub fn resolve_db_op(op: u8) -> &'static str {
    match op {
        0 => "SELECT",
        1 => "INSERT",
        2 => "UPDATE",
        3 => "DELETE",
        4 => "CALL",
        5 => "DDL",
        _ => "UNKNOWN",
    }
}

/// Resolve MQ op_type to human-readable string.
pub fn resolve_mq_op(op: u8) -> &'static str {
    match op {
        0 => "PUBLISH",
        1 => "CONSUME",
        2 => "ACK",
        3 => "NACK",
        4 => "DLQ",
        _ => "UNKNOWN",
    }
}

/// Resolve security event type.
pub fn resolve_security_event(t: u8) -> &'static str {
    match t {
        0 => "AUTH",
        1 => "AUTHZ",
        2 => "AUDIT",
        3 => "ANOMALY",
        4 => "INTRUSION",
        5 => "POLICY",
        _ => "UNKNOWN",
    }
}

/// Resolve security outcome.
pub fn resolve_security_outcome(o: u8) -> &'static str {
    match o {
        0 => "ALLOW",
        1 => "DENY",
        2 => "CHALLENGE",
        3 => "ERROR",
        _ => "UNKNOWN",
    }
}

/// Resolve system event type.
pub fn resolve_system_event(t: u8) -> &'static str {
    match t {
        0 => "METRICS",
        1 => "SIGNAL",
        2 => "OOM",
        3 => "PANIC",
        4 => "STARTUP",
        5 => "SHUTDOWN",
        _ => "UNKNOWN",
    }
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
