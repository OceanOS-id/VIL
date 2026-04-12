use serde_json::{json, Value};
fn tier1_faq_agent(input: &Value) -> Result<Value, String> {
    let q = input["question"].as_str().unwrap_or("").to_lowercase();
    if q.contains("password") { Ok(json!({"resolved": true, "answer": "Reset via Settings > Security", "tier": 1})) }
    else if q.contains("pricing") { Ok(json!({"resolved": true, "answer": "See /pricing page", "tier": 1})) }
    else { Ok(json!({"resolved": false, "answer": "ESCALATE", "tier": 1})) }
}
fn tier2_diagnostic_agent(input: &Value) -> Result<Value, String> {
    let q = input["question"].as_str().unwrap_or("").to_lowercase();
    if q.contains("slow") || q.contains("error") { Ok(json!({"resolved": true, "answer": "Diagnostics show: service healthy, check client-side", "tier": 2})) }
    else { Ok(json!({"resolved": false, "answer": "ESCALATE", "tier": 2})) }
}
fn tier3_incident_agent(input: &Value) -> Result<Value, String> {
    let q = input["question"].as_str().unwrap_or("");
    let incident_id = format!("INC-{:04x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() % 0xFFFF);
    Ok(json!({"resolved": true, "answer": format!("Incident {} created for: {}", incident_id, q), "tier": 3, "incident_id": incident_id}))
}
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/407-agent-multi-agent-orchestration/vwfd/workflows", 8080)
        .native("tier1_faq_agent", tier1_faq_agent)
        .native("tier2_diagnostic_agent", tier2_diagnostic_agent)
        .native("tier3_incident_agent", tier3_incident_agent).run().await;
}
