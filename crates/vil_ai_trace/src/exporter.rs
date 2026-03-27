//! Export trace data in various formats.

use crate::span::TraceSpan;
use serde_json;

/// Export a collection of spans as JSON.
pub fn export_json(spans: &[TraceSpan]) -> String {
    serde_json::to_string_pretty(spans).unwrap_or_else(|_| "[]".to_string())
}

/// Export as NDJSON (one JSON object per line).
pub fn export_ndjson(spans: &[TraceSpan]) -> String {
    spans
        .iter()
        .filter_map(|s| serde_json::to_string(s).ok())
        .collect::<Vec<_>>()
        .join("\n")
}
