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
//
// Version-aware: dispatches to version-specific resolvers based on
// VilLogHeader.version, allowing schema evolution without breaking
// old log resolution.
// =============================================================================

use crate::dict;
use crate::types::*;
use zerocopy::FromBytes;

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
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let months = [
        31,
        if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            29
        } else {
            28
        },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 1u32;
    for &days_in_month in &months {
        if remaining < days_in_month {
            break;
        }
        remaining -= days_in_month;
        m += 1;
    }
    let d = remaining + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y, m, d, hours, minutes, seconds, subsec_ms
    )
}

/// Resolve a LogSlot into a ResolvedLog with all human-readable fields.
///
/// Dispatches to version-specific detail resolvers based on the `version`
/// field in VilLogHeader. This enables schema evolution: when a v2 payload
/// layout is introduced, a new `resolve_*_detail_v2` function can be added
/// without breaking resolution of existing v1 logs.
pub fn resolve_slot(slot: &LogSlot) -> ResolvedLog {
    let h = &slot.header;
    let level = LogLevel::from(h.level);
    let category = LogCategory::from(h.category);
    let version = h.version;

    let detail = match (category, version) {
        (LogCategory::Db, 1) => resolve_db_detail_v1(&slot.payload),
        (LogCategory::Db, _) => format!("[unknown DB schema v{}]", version),
        (LogCategory::Mq, 1) => resolve_mq_detail_v1(&slot.payload),
        (LogCategory::Mq, _) => format!("[unknown MQ schema v{}]", version),
        (LogCategory::Access, 1) => resolve_access_detail_v1(&slot.payload),
        (LogCategory::Access, _) => format!("[unknown Access schema v{}]", version),
        (LogCategory::Ai, 1) => resolve_ai_detail_v1(&slot.payload),
        (LogCategory::Ai, _) => format!("[unknown AI schema v{}]", version),
        (LogCategory::System, 1) => resolve_system_detail_v1(&slot.payload),
        (LogCategory::System, _) => format!("[unknown System schema v{}]", version),
        (LogCategory::Security, 1) => resolve_security_detail_v1(&slot.payload),
        (LogCategory::Security, _) => format!("[unknown Security schema v{}]", version),
        (LogCategory::App, 1) => resolve_app_detail_v1(&slot.payload),
        (LogCategory::App, _) => format!("[unknown App schema v{}]", version),
    };

    ResolvedLog {
        timestamp: format_ts(h.timestamp_ns),
        level: format!("{}", level),
        category: format!("{}", category),
        service: resolve_hash(h.service_hash),
        handler: resolve_hash(h.handler_hash),
        node: resolve_hash(h.node_hash),
        process_id: h.process_id,
        trace_id: if h.trace_id == 0 {
            "-".into()
        } else {
            format!("{:016x}", h.trace_id)
        },
        detail,
    }
}

/// Format a LogSlot as a single human-readable line.
pub fn format_human(slot: &LogSlot) -> String {
    let r = resolve_slot(slot);
    format!(
        "{} {:>5} [{}] svc={} {} | {}",
        r.timestamp, r.level, r.category, r.service, r.handler, r.detail
    )
}

// ── Detail resolvers per category (v1) ──

fn resolve_db_detail_v1(payload: &[u8; 192]) -> String {
    let Ok((p, _rest)) = crate::types::DbPayload::read_from_prefix(payload.as_slice()) else {
        return "[malformed DB payload]".to_string();
    };
    let db = resolve_hash(p.db_hash);
    let table = resolve_hash(p.table_hash);
    let query = resolve_hash(p.query_hash);
    let op = dict::resolve_db_op(p.op_type);

    if p.error_code != 0 {
        format!(
            "{} {}.{} query={} dur={}ns rows={} ERROR({})",
            op, db, table, query, p.duration_ns, p.rows_affected, p.error_code
        )
    } else {
        format!(
            "{} {}.{} query={} dur={}ns rows={}",
            op, db, table, query, p.duration_ns, p.rows_affected
        )
    }
}

fn resolve_mq_detail_v1(payload: &[u8; 192]) -> String {
    let Ok((p, _rest)) = crate::types::MqPayload::read_from_prefix(payload.as_slice()) else {
        return "[malformed MQ payload]".to_string();
    };
    let broker = resolve_hash(p.broker_hash);
    let topic = resolve_hash(p.topic_hash);
    let op = dict::resolve_mq_op(p.op_type);

    format!(
        "{} {}/{} offset={} size={}B dur={}ns",
        op, broker, topic, p.offset, p.message_bytes, p.e2e_latency_ns
    )
}

fn resolve_access_detail_v1(payload: &[u8; 192]) -> String {
    let Ok((p, _rest)) = crate::types::AccessPayload::read_from_prefix(payload.as_slice()) else {
        return "[malformed Access payload]".to_string();
    };
    let method_str = match p.method {
        0 => "GET",
        1 => "POST",
        2 => "PUT",
        3 => "DELETE",
        4 => "PATCH",
        5 => "HEAD",
        6 => "OPTIONS",
        _ => "?",
    };

    format!(
        "{} {} dur={}ns req={}B resp={}B",
        method_str, p.status_code, p.duration_ns, p.request_bytes, p.response_bytes
    )
}

fn resolve_ai_detail_v1(payload: &[u8; 192]) -> String {
    let Ok((p, _rest)) = crate::types::AiPayload::read_from_prefix(payload.as_slice()) else {
        return "[malformed AI payload]".to_string();
    };
    let model = resolve_hash(p.model_hash);
    let provider = resolve_hash(p.provider_hash);

    format!(
        "{}/{} in={} out={} dur={}ms cost=${:.4}",
        provider,
        model,
        p.input_tokens,
        p.output_tokens,
        p.latency_ns / 1_000_000,
        p.cost_micro_usd as f64 / 1_000_000.0
    )
}

fn resolve_system_detail_v1(payload: &[u8; 192]) -> String {
    let Ok((p, _rest)) = crate::types::SystemPayload::read_from_prefix(payload.as_slice()) else {
        return "[malformed System payload]".to_string();
    };
    let event = dict::resolve_system_event(p.event_type);
    format!(
        "{} cpu={:.1}% mem={}MB",
        event,
        p.cpu_pct_x100 as f64 / 100.0,
        p.mem_kb / 1024
    )
}

fn resolve_security_detail_v1(payload: &[u8; 192]) -> String {
    let Ok((p, _rest)) = crate::types::SecurityPayload::read_from_prefix(payload.as_slice()) else {
        return "[malformed Security payload]".to_string();
    };
    let actor = resolve_hash(p.actor_hash);
    let resource = resolve_hash(p.resource_hash);
    let action = resolve_hash(p.action_hash);
    let event = dict::resolve_security_event(p.event_type);
    let result = dict::resolve_security_outcome(p.outcome);

    format!(
        "{} {} actor={} resource={} action={} risk={}",
        event, result, actor, resource, action, p.risk_score
    )
}

fn resolve_app_detail_v1(payload: &[u8; 192]) -> String {
    let Ok((p, _rest)) = crate::types::AppPayload::read_from_prefix(payload.as_slice()) else {
        return "[malformed App payload]".to_string();
    };
    let code = resolve_hash(p.code_hash);
    let kv_len = p.kv_len as usize;

    if kv_len > 0 && kv_len <= 184 {
        // Try decode MsgPack KV from kv_bytes
        if let Ok(val) = rmp_serde::from_slice::<serde_json::Value>(&p.kv_bytes[..kv_len]) {
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

    #[test]
    fn test_resolve_slot_unknown_version() {
        // Build a slot with version=99 — should produce "[unknown ... schema v99]"
        let mut slot = LogSlot::default();
        slot.header.version = 99;
        slot.header.category = LogCategory::Db as u8;
        let resolved = resolve_slot(&slot);
        assert!(
            resolved.detail.contains("unknown"),
            "got: {}",
            resolved.detail
        );
        assert!(resolved.detail.contains("v99"), "got: {}", resolved.detail);
    }

    #[test]
    fn test_resolve_slot_v1() {
        // Build a v1 DB slot
        let mut slot = LogSlot::default();
        slot.header.version = 1;
        slot.header.category = LogCategory::Db as u8;
        slot.header.level = LogLevel::Info as u8;
        // op_type = 0 (SELECT)
        slot.payload[20] = 0;
        let resolved = resolve_slot(&slot);
        assert!(
            resolved.detail.contains("SELECT"),
            "got: {}",
            resolved.detail
        );
    }
}
