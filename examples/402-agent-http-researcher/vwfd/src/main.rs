// 402 — HTTP Research Agent (VWFD)
// Business logic identical to standard:
//   POST /api/research/analyze — multi-turn agent loop (max 8), tools: fetch_products + calculator
//   Response: { answer, tools_used: Vec<String>, iterations }
use serde_json::{json, Value};

fn research_tool_executor(input: &Value) -> Result<Value, String> {
    let llm_output = input["llm_output"].as_str().unwrap_or("");

    if llm_output.contains("DONE:") {
        let answer = llm_output.split("DONE:").nth(1).unwrap_or("").trim();
        return Ok(json!({
            "done": true,
            "final_answer": answer,
            "tool_name": "none",
            "observation": ""
        }));
    }

    if let Some(start) = llm_output.find("<tool>") {
        if let Some(end) = llm_output.find("</tool>") {
            let tool_call = &llm_output[start + 6..end];
            if let Some(colon) = tool_call.find(':') {
                let tool_name = &tool_call[..colon];
                let tool_input = &tool_call[colon + 1..];

                let observation = match tool_name {
                    "fetch_products" => {
                        json!([
                            {"id": "P001", "name": "Widget Pro", "category": "tools", "price_cents": 2999, "stock": 150, "rating": 4.5},
                            {"id": "P002", "name": "Gadget X", "category": "electronics", "price_cents": 4999, "stock": 75, "rating": 4.2},
                            {"id": "P003", "name": "Super Widget", "category": "tools", "price_cents": 5499, "stock": 30, "rating": 4.8}
                        ]).to_string()
                    }
                    "calculator" => {
                        let parts: Vec<&str> = tool_input.trim().splitn(3, ' ').collect();
                        if parts.len() == 3 {
                            let a = parts[0].parse::<f64>().unwrap_or(0.0);
                            let b = parts[2].parse::<f64>().unwrap_or(0.0);
                            let r = match parts[1] {
                                "+" => a + b, "-" => a - b, "*" => a * b,
                                "/" => if b != 0.0 { a / b } else { 0.0 },
                                _ => 0.0
                            };
                            format!("{}", r)
                        } else {
                            tool_input.to_string()
                        }
                    }
                    _ => format!("Unknown tool: {}", tool_name),
                };

                return Ok(json!({
                    "done": false,
                    "tool_name": tool_name,
                    "observation": observation,
                    "final_answer": ""
                }));
            }
        }
    }

    Ok(json!({
        "done": false,
        "tool_name": "unknown",
        "observation": "Could not parse tool call from LLM output",
        "final_answer": ""
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/402-agent-http-researcher/vwfd/workflows", 8080)
        .native("research_tool_executor", research_tool_executor)
        .run().await;
}
