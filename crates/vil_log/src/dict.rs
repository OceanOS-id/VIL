// =============================================================================
// vil_log::dict — DictRegistry
// =============================================================================
//
// Maps (category: u8, hash: u32) → String for reverse lookup.
// Uses rustc-hash (FxHasher) internally for collision detection, truncated to u32
// for the wire format.
// Thread-safe via std::sync::Mutex (simple, v0.1).
// =============================================================================

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

fn hash64(bytes: &[u8]) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Dictionary entry storing the original string and full 64-bit hash
/// for collision detection.
#[derive(Debug, Clone)]
struct DictEntry {
    value: String,
    hash64: u64,
}

/// Global dictionary: hash -> DictEntry for reverse lookup.
static DICT: Mutex<Option<HashMap<u32, DictEntry>>> = Mutex::new(None);

/// Compute a 32-bit FxHash of a string and register it in the global dict.
///
/// Internally uses 64-bit hash for collision detection, truncated to u32
/// for the wire format. If a collision is detected (different string, same u32),
/// a warning is printed via eprintln.
///
/// Returns the hash, which can be stored in log headers for compact storage.
pub fn register_str(s: &str) -> u32 {
    let h64 = hash64(s.as_bytes());
    let h32 = h64 as u32; // truncate for wire format

    let mut guard = DICT.lock().unwrap_or_else(|e| e.into_inner());
    let dict = guard.get_or_insert_with(HashMap::new);

    match dict.entry(h32) {
        std::collections::hash_map::Entry::Occupied(existing) => {
            if existing.get().hash64 != h64 {
                // COLLISION DETECTED — different string, same u32 hash
                eprintln!(
                    "[vil_log WARNING] Hash collision: '{}' and '{}' both hash to 0x{:08x}",
                    existing.get().value,
                    s,
                    h32
                );
            }
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(DictEntry {
                value: s.to_string(),
                hash64: h64,
            });
        }
    }

    h32
}

/// Look up a string by its hash. Returns None if not registered.
pub fn lookup(hash: u32) -> Option<String> {
    let guard = DICT.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|d| d.get(&hash).map(|e| e.value.clone()))
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
/// Format: `{ "hash_decimal": { "value": "original_string", "hash64": 12345 }, ... }`
pub fn save_to_file(path: &std::path::Path) -> std::io::Result<()> {
    let guard = DICT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(dict) = guard.as_ref() {
        let map: std::collections::BTreeMap<String, serde_json::Value> = dict
            .iter()
            .map(|(k, entry)| {
                (
                    format!("{}", k),
                    serde_json::json!({
                        "value": entry.value,
                        "hash64": entry.hash64
                    }),
                )
            })
            .collect();
        let json = serde_json::to_string_pretty(&map)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)?;
    }
    Ok(())
}

/// Load dictionary from a JSON file (merges with existing entries).
///
/// Supports both new format (object with "value" and "hash64" fields) and
/// old format (plain string value) for backward compatibility.
pub fn load_from_file(path: &std::path::Path) -> std::io::Result<usize> {
    let json = std::fs::read_to_string(path)?;
    let raw: std::collections::BTreeMap<String, serde_json::Value> = serde_json::from_str(&json)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut guard = DICT.lock().unwrap_or_else(|e| e.into_inner());
    let dict = guard.get_or_insert_with(HashMap::new);
    let mut loaded = 0;
    for (hash_str, val) in raw {
        if let Ok(hash) = hash_str.parse::<u32>() {
            let (value, h64) = if let Some(obj) = val.as_object() {
                // New format: { "value": "...", "hash64": ... }
                let value = obj
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let h64 = obj
                    .get("hash64")
                    .and_then(|v| v.as_u64())
                    .unwrap_or_else(|| hash64(value.as_bytes()));
                (value, h64)
            } else if let Some(s) = val.as_str() {
                // Old format: plain string value — recompute hash64
                let value = s.to_string();
                let h64 = hash64(value.as_bytes());
                (value, h64)
            } else {
                continue;
            };
            dict.entry(hash).or_insert_with(|| {
                loaded += 1;
                DictEntry { value, hash64: h64 }
            });
        }
    }
    Ok(loaded)
}

/// Export the full dictionary as a HashMap (for external use).
pub fn export_all() -> HashMap<u32, String> {
    let guard = DICT.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .map(|d| d.iter().map(|(k, e)| (*k, e.value.clone())).collect())
        .unwrap_or_default()
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

    #[test]
    fn test_dict_size() {
        let before = dict_size();
        register_str("dict_size_test_unique_key");
        let after = dict_size();
        assert!(after >= before + 1);
    }

    #[test]
    fn test_export_all() {
        register_str("export_test_val");
        let exported = export_all();
        assert!(exported.values().any(|v| v == "export_test_val"));
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("vil_log_dict_test.json");

        register_str("roundtrip_test_str");
        save_to_file(&path).unwrap();

        // Verify file is valid JSON and loadable
        let loaded = load_from_file(&path).unwrap();
        // loaded may be 0 if entries already existed, but no error
        let _ = loaded;

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_old_format() {
        let dir = std::env::temp_dir();
        let path = dir.join("vil_log_dict_old_format_test.json");

        // Write old format: plain string values
        let old_data = serde_json::json!({
            "12345": "old_format_value"
        });
        std::fs::write(&path, serde_json::to_string_pretty(&old_data).unwrap()).unwrap();

        let loaded = load_from_file(&path).unwrap();
        assert!(loaded >= 1);

        // Verify the value is accessible
        let found = lookup(12345);
        assert_eq!(found.as_deref(), Some("old_format_value"));

        let _ = std::fs::remove_file(&path);
    }
}
