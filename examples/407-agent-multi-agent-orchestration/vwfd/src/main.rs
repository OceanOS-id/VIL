// 407 — Multi-Agent Orchestration: Tiered Support (VWFD)
// Business logic identical to standard:
//   POST /api/support/ask — 3-tier escalation (FAQ → Diagnostic → Incident)
//   Response: { resolved_by, answer, tiers_involved: [{tier, response}], total_ms }
use serde_json::{json, Value};

fn tiers_handler(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "escalation_path": [
            {"tier": 1, "name": "FAQ Agent", "tools": ["faq_lookup"], "handles": "Common questions: password reset, billing, subscriptions", "escalation_trigger": "confidence < 0.7"},
            {"tier": 2, "name": "Technical Support Agent", "tools": ["run_diagnostic", "search_logs"], "handles": "System issues, performance problems, error investigation", "escalation_trigger": "confidence < 0.6"},
            {"tier": 3, "name": "Incident Manager", "tools": ["create_incident", "execute_runbook"], "handles": "Critical/unresolved issues, incident creation", "escalation_trigger": "terminal — always resolves"}
        ]
    }))
}

fn tier1_faq_agent(input: &Value) -> Result<Value, String> {
    let q = input["question"].as_str().unwrap_or("").to_lowercase();
    if q.contains("password") || q.contains("reset") {
        Ok(json!({"resolved": true, "answer": "You can reset your password at Settings > Security > Change Password. If locked out, use the 'Forgot Password' link on the login page.", "confidence": 0.92, "matched_faq_id": "FAQ-003"}))
    } else if q.contains("pricing") || q.contains("plan") || q.contains("cost") {
        Ok(json!({"resolved": true, "answer": "See our pricing page at /pricing. Plans: Starter ($9/mo), Pro ($29/mo), Enterprise (custom). All include 14-day free trial.", "confidence": 0.88, "matched_faq_id": "FAQ-007"}))
    } else if q.contains("cancel") || q.contains("subscription") {
        Ok(json!({"resolved": true, "answer": "To cancel your subscription, go to Settings > Billing > Cancel Plan. Cancellation takes effect at end of current billing period.", "confidence": 0.85, "matched_faq_id": "FAQ-012"}))
    } else {
        Ok(json!({"resolved": false, "answer": "Unable to find a matching FAQ for this issue.", "confidence": 0.2, "matched_faq_id": null}))
    }
}

fn tier2_diagnostic_agent(input: &Value) -> Result<Value, String> {
    let q = input["question"].as_str().unwrap_or("").to_lowercase();
    if q.contains("slow") || q.contains("latency") || q.contains("timeout") {
        Ok(json!({"resolved": true, "answer": "Diagnostics complete: API latency elevated (p95: 2.3s). Root cause: database connection pool exhaustion. Recommended fix: increase pool size from 10 to 25.", "diagnosis": "connection_pool_exhaustion", "health_status": "degraded", "log_matches": 3}))
    } else if q.contains("error") || q.contains("500") || q.contains("crash") {
        Ok(json!({"resolved": true, "answer": "Log analysis found 47 matching errors in last 24h. Pattern: NullPointerException in UserService.getProfile(). Fix: null check added in hotfix branch.", "diagnosis": "null_pointer_exception", "health_status": "error", "log_matches": 47}))
    } else {
        Ok(json!({"resolved": false, "answer": "Diagnostic tools found no known issues matching this description. Escalating to incident management.", "diagnosis": "unknown", "health_status": "healthy", "log_matches": 0}))
    }
}

fn tier3_incident_agent(input: &Value) -> Result<Value, String> {
    let q = input["question"].as_str().unwrap_or("");
    let incident_id = format!("INC-{:04X}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() % 0xFFFF);
    Ok(json!({
        "resolved": true,
        "answer": format!("Incident {} created. Severity: P2. Assigned to on-call team. Runbook 'general-triage' executed. ETA: 30 minutes.", incident_id),
        "incident_id": incident_id,
        "severity": "P2",
        "runbook_executed": "general-triage",
        "runbook_result": "executed",
        "assigned_team": "oncall-platform",
        "eta_minutes": 30
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/407-agent-multi-agent-orchestration/vwfd/workflows", 8080)
        .native("tier1_faq_agent", tier1_faq_agent)
        .native("tier2_diagnostic_agent", tier2_diagnostic_agent)
        .native("tier3_incident_agent", tier3_incident_agent)
        .native("tiers_handler", tiers_handler)
        .run().await;
}
