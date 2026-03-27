// =============================================================================
// vil_log::resolve — Human-Readable Log Resolution
// =============================================================================
//
// Converts raw LogSlot (hashes, numeric codes) into human-readable strings.
//
// Two modes:
//   1. resolve_slot()  — returns structured ResolvedLog
//   2. format_human()  — returns single-line human-readable string
//
// Uses the global DictRegistry for hash→string lookup.
// Unknown hashes display as hex (e.g., "0x720c0265").
// =============================================================================

use crate::dict;
use crate::types::*;

/// Resolved log entry — all fields as human-readable strings.
#[derive(Debug, Clone)]
pub struct ResolvedLog {
    pub timestamp: String,
    pub level: String,
    pub category: String,
    pub service: String,
    pub handler: String,
    pub node: String,
    pub process_id: u64,
    pub trace_id: String,
    pub detail: String,
}

/// Resolve a hash to its registered string, or format as hex.
fn resolve_hash(hash: u32) -> String {
    if hash == 0 {
        return "-".to_string();
    }
    dict::lookup(hash).unwrap_or_else(|| format!("0x{:08x}", hash))
}

/// Format nanosecond timestamp to ISO-8601 datetime.
fn format_ts(ns: u64) -> String {
    let secs = ns / 1_000_000_000;
    let subsec_ms = (ns % 1_000_000_000) / 1_000_000;
    // Simple UTC formatting (no chrono dependency needed)
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Approximate date calculation (good enough for log display)
    let mut y = 1970i64;
    let mut remaining = days_since_epoch as i64;
    loop {
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if remaining < days_in_year { break; }
        remaining -= days_in_year;
        y += 1;
    }
    let months = [31, if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 29 } else { 28 },
                  31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 1u32;
    for &days_in_month in &months {
        if remaining < days_in_month { break; }
        remaining -= days_in_month;
        m += 1;
    }
    let d = remaining + 1;

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y, m, d, hours, minutes, seconds, subsec_ms)
}

/// Resolve a LogSlot into a ResolvedLog with all human-readable fields.
pub fn resolve_slot(slot: &LogSlot) -> ResolvedLog {
    let h = &slot.header;
    let level = LogLevel::from(h.level);
    let category = LogCategory::from(h.category);

    let detail = match category {
        LogCategory::Db => resolve_db_detail(&slot.payload),
        LogCategory::Mq => resolve_mq_detail(&slot.payload),
        LogCategory::Access => resolve_access_detail(&slot.payload),
        LogCategory::Ai => resolve_ai_detail(&slot.payload),
        LogCategory::System => resolve_system_detail(&slot.payload),
        LogCategory::Security => resolve_security_detail(&slot.payload),
        LogCategory::App => resolve_app_detail(&slot.payload),
    };

    ResolvedLog {
        timestamp: format_ts(h.timestamp_ns),
        level: format!("{}", level),
        category: format!("{}", category),
        service: resolve_hash(h.service_hash),
        handler: resolve_hash(h.handler_hash),
        node: resolve_hash(h.node_hash),
        process_id: h.process_id,
        trace_id: if h.trace_id == 0 { "-".into() } else { format!("{:016x}", h.trace_id) },
        detail,
    }
}

/// Format a LogSlot as a single human-readable line.
pub fn format_human(slot: &LogSlot) -> String {
    let r = resolve_slot(slot);
    format!("{} {:>5} [{}] svc={} {} | {}",
        r.timestamp, r.level, r.category, r.service, r.handler, r.detail)
}

// ── Detail resolvers per category ──

fn resolve_db_detail(payload: &[u8; 192]) -> String {
    // DbPayload layout: db_hash(4) + table_hash(4) + query_hash(4) + duration_us(4) + rows(4) + op(1) + ...
    let db_hash = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let table_hash = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let query_hash = u32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]);
    let duration_us = u32::from_le_bytes([payload[12], payload[13], payload[14], payload[15]]);
    let rows = u32::from_le_bytes([payload[16], payload[17], payload[18], payload[19]]);
    let op_type = payload[20];
    let error_code = payload[23];

    let db = resolve_hash(db_hash);
    let table = resolve_hash(table_hash);
    let query = resolve_hash(query_hash);
    let op = dict::resolve_db_op(op_type);

    if error_code != 0 {
        format!("{} {}.{} query={} dur={}us rows={} ERROR({})",
            op, db, table, query, duration_us, rows, error_code)
    } else {
        format!("{} {}.{} query={} dur={}us rows={}",
            op, db, table, query, duration_us, rows)
    }
}

fn resolve_mq_detail(payload: &[u8; 192]) -> String {
    let broker_hash = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let topic_hash = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let _group_hash = u32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]);
    let offset = u64::from_le_bytes([payload[12], payload[13], payload[14], payload[15],
                                      payload[16], payload[17], payload[18], payload[19]]);
    let msg_bytes = u32::from_le_bytes([payload[20], payload[21], payload[22], payload[23]]);
    let latency_us = u32::from_le_bytes([payload[24], payload[25], payload[26], payload[27]]);
    let op_type = payload[28];

    let broker = resolve_hash(broker_hash);
    let topic = resolve_hash(topic_hash);
    let op = dict::resolve_mq_op(op_type);

    format!("{} {}/{} offset={} size={}B dur={}us",
        op, broker, topic, offset, msg_bytes, latency_us)
}

fn resolve_access_detail(payload: &[u8; 192]) -> String {
    let method = payload[0];
    let status = u16::from_le_bytes([payload[1], payload[2]]);
    let _protocol = payload[3];
    let duration_us = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let req_bytes = u32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]);
    let resp_bytes = u32::from_le_bytes([payload[12], payload[13], payload[14], payload[15]]);

    let method_str = match method {
        0 => "GET", 1 => "POST", 2 => "PUT", 3 => "DELETE",
        4 => "PATCH", 5 => "HEAD", 6 => "OPTIONS", _ => "?",
    };

    format!("{} {} dur={}us req={}B resp={}B",
        method_str, status, duration_us, req_bytes, resp_bytes)
}

fn resolve_ai_detail(payload: &[u8; 192]) -> String {
    let model_hash = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let provider_hash = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let input_tokens = u32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]);
    let output_tokens = u32::from_le_bytes([payload[12], payload[13], payload[14], payload[15]]);
    let latency_us = u32::from_le_bytes([payload[16], payload[17], payload[18], payload[19]]);
    let cost = u32::from_le_bytes([payload[20], payload[21], payload[22], payload[23]]);

    let model = resolve_hash(model_hash);
    let provider = resolve_hash(provider_hash);

    format!("{}/{} in={} out={} dur={}ms cost=${:.4}",
        provider, model, input_tokens, output_tokens,
        latency_us / 1000, cost as f64 / 1_000_000.0)
}

fn resolve_system_detail(payload: &[u8; 192]) -> String {
    let cpu_x100 = u16::from_le_bytes([payload[0], payload[1]]);
    let mem_kb = u32::from_le_bytes([payload[2], payload[3], payload[4], payload[5]]);
    let event_type = payload[22];

    let event = dict::resolve_system_event(event_type);
    format!("{} cpu={:.1}% mem={}MB",
        event, cpu_x100 as f64 / 100.0, mem_kb / 1024)
}

fn resolve_security_detail(payload: &[u8; 192]) -> String {
    let actor_hash = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let resource_hash = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let action_hash = u32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]);
    let event_type = payload[16];
    let outcome = payload[17];
    let risk = payload[18];

    let actor = resolve_hash(actor_hash);
    let resource = resolve_hash(resource_hash);
    let action = resolve_hash(action_hash);
    let event = dict::resolve_security_event(event_type);
    let result = dict::resolve_security_outcome(outcome);

    format!("{} {} actor={} resource={} action={} risk={}",
        event, result, actor, resource, action, risk)
}

fn resolve_app_detail(payload: &[u8; 192]) -> String {
    let code_hash = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let kv_len = u16::from_le_bytes([payload[4], payload[5]]) as usize;

    let code = resolve_hash(code_hash);

    if kv_len > 0 && kv_len <= 184 {
        // Try decode MsgPack KV from payload[8..8+kv_len]
        if let Ok(val) = rmp_serde::from_slice::<serde_json::Value>(&payload[8..8 + kv_len]) {
            if let Ok(json) = serde_json::to_string(&val) {
                return format!("{} {}", code, json);
            }
        }
    }
    format!("{}", code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_ts() {
        // 2026-03-27 roughly
        let ts = 1774627490_000_000_000u64;
        let s = format_ts(ts);
        assert!(s.starts_with("2026-"), "got: {}", s);
        assert!(s.ends_with("Z"));
    }

    #[test]
    fn test_resolve_hash_known() {
        let h = dict::register_str("test-service");
        let resolved = resolve_hash(h);
        assert_eq!(resolved, "test-service");
    }

    #[test]
    fn test_resolve_hash_unknown() {
        let resolved = resolve_hash(0xDEADBEEF);
        assert_eq!(resolved, "0xdeadbeef");
    }
}
