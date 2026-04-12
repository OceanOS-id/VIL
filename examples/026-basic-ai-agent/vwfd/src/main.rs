use serde_json::{json, Value};
fn react_agent_loop(input: &Value) -> Result<Value, String> {
    let question = input["question"].as_str().unwrap_or("");
    // Simplified ReAct: 3 tools, return mock result
    let tools_used = vec!["system_status", "knowledge_base"];
    Ok(json!({
        "answer": format!("Based on analysis of your question '{}', the system is healthy and documentation suggests checking the config.", question),
        "tools_used": tools_used,
        "iterations": 2,
        "trace": ["Thought: need system status", "Action: system_status", "Observation: all green", "Thought: check KB", "Action: knowledge_base", "Observation: found docs", "FINAL_ANSWER"]
    }))
}
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/026-basic-ai-agent/vwfd/workflows", 8080)
        .native("react_agent_loop", react_agent_loop).run().await;
}
