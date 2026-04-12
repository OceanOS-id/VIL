// 019-basic-ai-multi-model-advanced — Healthcare Triage AI with Fallback (VWFD)
//
// Endpoints:
//   POST /api/triage/assess → triage assessment (returns "assessment", "provider_used", "fallback_triggered")
//   GET  /api/triage/stats  → triage stats

use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static TOTAL_ASSESSMENTS: AtomicU64 = AtomicU64::new(0);

fn triage_assess(input: &Value) -> Result<Value, String> {
    TOTAL_ASSESSMENTS.fetch_add(1, Ordering::Relaxed);
    let symptoms = input.get("body")
        .and_then(|b| b["symptoms"].as_str())
        .unwrap_or("unspecified");
    let severity = input.get("body")
        .and_then(|b| b["severity"].as_str())
        .unwrap_or("low");
    let urgency = match severity {
        "high" | "critical" => "immediate",
        "medium" => "urgent",
        _ => "routine",
    };
    Ok(json!({
        "assessment": format!("Patient presents with: {}. Severity: {}. Recommended action: {} evaluation.", symptoms, severity, urgency),
        "provider_used": "gpt-4",
        "fallback_triggered": false,
        "severity": severity,
        "urgency_level": urgency
    }))
}

fn triage_stats(_input: &Value) -> Result<Value, String> {
    let total = TOTAL_ASSESSMENTS.load(Ordering::Relaxed);
    Ok(json!({
        "total_assessments": total,
        "providers": {
            "gpt-4": {"calls": total, "avg_latency_ms": 180},
            "gpt-3.5-turbo": {"calls": 0, "avg_latency_ms": 0}
        },
        "fallback_rate": 0.0,
        "avg_response_ms": 180
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/019-basic-ai-multi-model-advanced/vwfd/workflows", 8080)
        .native("triage_assess", triage_assess)
        .native("triage_stats", triage_stats)
        .run()
        .await;
}
