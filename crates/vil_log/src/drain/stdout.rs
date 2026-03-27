// =============================================================================
// vil_log::drain::stdout — StdoutDrain
// =============================================================================
//
// Formats LogSlot to stdout. Three modes:
//   Pretty  — multi-line colored human-readable (ANSI escape codes)
//   Compact — single-line colored
//   Json    — JSON Lines (one JSON object per event, no color)
// =============================================================================

use std::io::Write;

use async_trait::async_trait;

use crate::drain::traits::LogDrain;
use crate::types::{LogCategory, LogLevel, LogSlot};

const RESET: &str  = "\x1b[0m";
const BOLD:  &str  = "\x1b[1m";
const DIM:   &str  = "\x1b[2m";

/// Output format for `StdoutDrain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdoutFormat {
    Pretty,
    Compact,
    Json,
}

/// Drain that writes to stdout with optional ANSI coloring.
pub struct StdoutDrain {
    format: StdoutFormat,
}

impl StdoutDrain {
    pub fn new(format: StdoutFormat) -> Self {
        Self { format }
    }

    pub fn pretty()  -> Self { Self::new(StdoutFormat::Pretty) }
    pub fn compact() -> Self { Self::new(StdoutFormat::Compact) }
    pub fn json()    -> Self { Self::new(StdoutFormat::Json) }
}

impl Default for StdoutDrain {
    fn default() -> Self {
        Self::pretty()
    }
}

fn format_slot_compact(slot: &LogSlot, out: &mut impl Write) {
    let level    = LogLevel::from(slot.header.level);
    let category = LogCategory::from(slot.header.category);
    let color    = level.ansi_color();
    let ts_ms    = slot.header.timestamp_ns / 1_000_000;

    let _ = writeln!(
        out,
        "{}{}{} {}[{}]{} ts={} svc={:08x} pid={}",
        color, BOLD, level, RESET,
        category, RESET,
        ts_ms,
        slot.header.service_hash,
        slot.header.process_id,
    );
}

fn format_slot_pretty(slot: &LogSlot, out: &mut impl Write) {
    let level    = LogLevel::from(slot.header.level);
    let category = LogCategory::from(slot.header.category);
    let color    = level.ansi_color();
    let ts_ms    = slot.header.timestamp_ns / 1_000_000;

    let _ = writeln!(out, "{}{}┌── {} [{}]{}", color, BOLD, level, category, RESET);
    let _ = writeln!(out, "{}│  timestamp : {}ms", DIM, ts_ms);
    let _ = writeln!(out, "{}│  service   : {:08x}", DIM, slot.header.service_hash);
    let _ = writeln!(out, "{}│  handler   : {:08x}", DIM, slot.header.handler_hash);
    let _ = writeln!(out, "{}│  pid       : {}", DIM, slot.header.process_id);
    let _ = writeln!(out, "{}│  trace_id  : {:016x}", DIM, slot.header.trace_id);

    // Attempt msgpack decode of payload
    if slot.payload[0] != 0 {
        if let Ok(val) = rmp_serde::from_slice::<serde_json::Value>(&slot.payload) {
            if let Ok(pretty) = serde_json::to_string_pretty(&val) {
                for line in pretty.lines() {
                    let _ = writeln!(out, "{}│  {}", DIM, line);
                }
            }
        }
    }

    let _ = writeln!(out, "{}└──{}", color, RESET);
}

fn format_slot_json(slot: &LogSlot, out: &mut impl Write) {
    let level    = LogLevel::from(slot.header.level);
    let category = LogCategory::from(slot.header.category);

    let mut map = serde_json::json!({
        "ts":       slot.header.timestamp_ns,
        "level":    level.to_string(),
        "category": category.to_string(),
        "svc":      slot.header.service_hash,
        "handler":  slot.header.handler_hash,
        "pid":      slot.header.process_id,
        "trace_id": slot.header.trace_id,
    });

    // Try to decode payload as msgpack
    if slot.payload[0] != 0 {
        if let Ok(val) = rmp_serde::from_slice::<serde_json::Value>(&slot.payload) {
            if let Some(obj) = map.as_object_mut() {
                obj.insert("data".to_string(), val);
            }
        }
    }

    let _ = writeln!(out, "{}", map);
}

#[async_trait]
impl LogDrain for StdoutDrain {
    fn name(&self) -> &'static str {
        "stdout"
    }

    async fn flush(&mut self, batch: &[LogSlot]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();

        for slot in batch {
            match self.format {
                StdoutFormat::Pretty  => format_slot_pretty(slot, &mut handle),
                StdoutFormat::Compact => format_slot_compact(slot, &mut handle),
                StdoutFormat::Json    => format_slot_json(slot, &mut handle),
            }
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}
