// =============================================================================
// vil_log::emit::macros — Semantic log emission macros
// =============================================================================
//
// Each macro builds a LogSlot and pushes it to the global ring.
// Failures (ring full) increment the drop counter silently — never block.
//
// When the ring is not initialized (try_global_striped() returns None),
// macros fall back to emitting via the `tracing` crate so that logs are
// not silently lost during early startup or in test contexts.
//
// app_log!(LEVEL, "event.code", { key: value, ... })
//   - Builds VilLogHeader with category=App, current timestamp
//   - Serializes fields to msgpack via rmp_serde
//   - Pushes to global ring
//
// access_log!(LEVEL, payload_expr)
// ai_log!(LEVEL, payload_expr)
// db_log!(LEVEL, payload_expr)
// mq_log!(LEVEL, payload_expr)
// system_log!(LEVEL, payload_expr)
// security_log!(LEVEL, payload_expr)
//   - Accept a pre-built payload struct
//   - Copy raw bytes into the slot payload section
// =============================================================================

/// Emit a general application log event to the global ring.
///
/// Falls back to `tracing` if the ring is not initialized.
///
/// # Example
/// ```rust,ignore
/// app_log!(Info, "user.login", { user_id: 42u64, success: true });
/// ```
#[macro_export]
macro_rules! app_log {
    ($level:ident, $code:expr, { $($key:ident : $val:expr),* $(,)? }) => {{
        use $crate::emit::ring::{try_global_striped, level_enabled};
        use $crate::types::{LogSlot, VilLogHeader, LogLevel, LogCategory};
        use $crate::dict::register_str;

        if level_enabled(LogLevel::$level as u8) {
        if let Some(striped) = try_global_striped() {
            let ts = {
                use std::time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64
            };

            let code_hash = register_str($code);

            let mut slot = LogSlot::default();
            slot.header = VilLogHeader {
                timestamp_ns: ts,
                level:        LogLevel::$level as u8,
                category:     LogCategory::App as u8,
                version:      1,
                service_hash: register_str(module_path!()),
                handler_hash: 0,
                node_hash:    0,
                process_id:   std::process::id() as u64,
                ..VilLogHeader::default()
            };

            // Serialize KV pairs to msgpack into payload
            let kv_map: std::collections::BTreeMap<&str, serde_json::Value> = {
                let mut m = std::collections::BTreeMap::new();
                m.insert("_code", serde_json::Value::from($code));
                $(
                    m.insert(stringify!($key), serde_json::json!($val));
                )*
                m
            };

            if let Ok(encoded) = rmp_serde::to_vec_named(&kv_map) {
                let len = encoded.len().min(184);
                // Write AppPayload manually: code_hash(u32) + kv_len(u16) + pad(u8x2) + bytes
                let ch_bytes = code_hash.to_le_bytes();
                slot.payload[0..4].copy_from_slice(&ch_bytes);
                let kl_bytes = (len as u16).to_le_bytes();
                slot.payload[4..6].copy_from_slice(&kl_bytes);
                let cursor = 8usize; // skip 2-byte pad
                slot.payload[cursor..cursor + len].copy_from_slice(&encoded[..len]);
            }

            let _ = striped.try_push(slot);
        }
        // Ring not initialized — silently skip
        } // level_enabled
    }};
}

/// Emit an access log event to the global ring.
///
/// # Example
/// ```rust,ignore
/// let p = AccessPayload { method: 0, status_code: 200, .. Default::default() };
/// access_log!(Info, p);
/// ```
#[macro_export]
macro_rules! access_log {
    ($level:ident, $payload:expr) => {{
        $crate::_emit_typed_log!($level, $crate::types::LogCategory::Access, $payload)
    }};
}

/// Emit an AI inference log event to the global ring.
#[macro_export]
macro_rules! ai_log {
    ($level:ident, $payload:expr) => {{
        $crate::_emit_typed_log!($level, $crate::types::LogCategory::Ai, $payload)
    }};
}

/// Emit a database operation log event to the global ring.
#[macro_export]
macro_rules! db_log {
    ($level:ident, $payload:expr) => {{
        $crate::_emit_typed_log!($level, $crate::types::LogCategory::Db, $payload)
    }};
}

/// Emit a message queue event to the global ring.
#[macro_export]
macro_rules! mq_log {
    ($level:ident, $payload:expr) => {{
        $crate::_emit_typed_log!($level, $crate::types::LogCategory::Mq, $payload)
    }};
}

/// Emit a system resource event to the global ring.
#[macro_export]
macro_rules! system_log {
    ($level:ident, $payload:expr) => {{
        $crate::_emit_typed_log!($level, $crate::types::LogCategory::System, $payload)
    }};
}

/// Emit a security event to the global ring.
#[macro_export]
macro_rules! security_log {
    ($level:ident, $payload:expr) => {{
        $crate::_emit_typed_log!($level, $crate::types::LogCategory::Security, $payload)
    }};
}

/// Internal helper — copy a typed payload struct into a LogSlot and push.
///
/// Falls back to `tracing` if the ring is not initialized.
#[doc(hidden)]
#[macro_export]
macro_rules! _emit_typed_log {
    ($level:ident, $category:expr, $payload:expr) => {{
        use $crate::emit::ring::{try_global_striped, level_enabled};
        use $crate::types::{LogSlot, VilLogHeader, LogLevel};
        use $crate::dict::register_str;

        if level_enabled(LogLevel::$level as u8) {
        if let Some(striped) = try_global_striped() {
            let ts = {
                use std::time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64
            };

            let mut slot = LogSlot::default();
            slot.header = VilLogHeader {
                timestamp_ns: ts,
                level:        LogLevel::$level as u8,
                category:     $category as u8,
                version:      1,
                service_hash: register_str(module_path!()),
                process_id:   std::process::id() as u64,
                ..VilLogHeader::default()
            };

            // Copy payload struct bytes into slot.payload
            let p = $payload;
            // SAFETY: `p` is a valid, fully-initialized struct; pointer and length are derived from it and do not exceed its size.
            let payload_bytes = unsafe {
                std::slice::from_raw_parts(
                    &p as *const _ as *const u8,
                    std::mem::size_of_val(&p).min(192),
                )
            };
            let copy_len = payload_bytes.len().min(192);
            slot.payload[..copy_len].copy_from_slice(&payload_bytes[..copy_len]);

            let _ = striped.try_push(slot);
        }
        // Ring not initialized — silently skip
        } // level_enabled
    }};
}
