// 405 — ReAct Multi-Tool Agent (VWFD)
// Business logic identical to standard:
//   POST /api/react — ReAct loop (max 5): Thought → Action → Observation
//   Tools: search (mock KB), calculator (eval)
//   Response: { answer, reasoning_trace, iterations, max_iter_reached }
use serde_json::{json, Value};

fn react_tool_dispatcher(input: &Value) -> Result<Value, String> {
    let llm_output = input["llm_output"].as_str().unwrap_or("");

    if llm_output.contains("FINAL_ANSWER:") {
        let answer = llm_output.split("FINAL_ANSWER:").nth(1).unwrap_or("").trim();
        let thought = extract_field(llm_output, "Thought:");
        return Ok(json!({
            "done": true,
            "thought": thought,
            "action": null,
            "action_input": null,
            "observation": null,
            "final_answer": answer
        }));
    }

    let thought = extract_field(llm_output, "Thought:");
    let action = extract_field(llm_output, "Action:");
    let action_input = extract_field(llm_output, "Action Input:");

    let observation = match action.as_str() {
        "search" => {
            let kb = [
                ("Indonesia GDP", "Indonesia's GDP is approximately $1.32 trillion (2023). Population: 275 million. GDP per capita: ~$4,800."),
                ("Japan GDP", "Japan's GDP is approximately $4.23 trillion (2023). Population: 125 million. GDP per capita: ~$33,800."),
                ("Singapore GDP", "Singapore's GDP is approximately $397 billion (2023). Population: 5.9 million. GDP per capita: ~$67,300."),
                ("United States GDP", "US GDP is approximately $26.95 trillion (2023). Population: 335 million. GDP per capita: ~$80,400."),
            ];
            let q = action_input.to_lowercase();
            kb.iter()
                .find(|(k, _)| q.contains(&k.to_lowercase()))
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| format!("No results found for: {}", action_input))
        }
        "calculator" => {
            let parts: Vec<&str> = action_input.trim().splitn(3, ' ').collect();
            if parts.len() == 3 {
                let a = parts[0].replace(',', "").parse::<f64>().unwrap_or(0.0);
                let b = parts[2].replace(',', "").parse::<f64>().unwrap_or(0.0);
                let r = match parts[1] {
                    "+" => a + b, "-" => a - b, "*" => a * b,
                    "/" => if b != 0.0 { a / b } else { 0.0 },
                    _ => 0.0
                };
                format!("{}", r)
            } else {
                format!("Could not evaluate: {}", action_input)
            }
        }
        _ => format!("Unknown action: {}", action),
    };

    Ok(json!({
        "done": false,
        "thought": thought,
        "action": action,
        "action_input": action_input,
        "observation": observation,
        "final_answer": ""
    }))
}

fn extract_field(text: &str, prefix: &str) -> String {
    text.lines()
        .find(|l| l.trim().starts_with(prefix))
        .map(|l| l.trim().strip_prefix(prefix).unwrap_or("").trim().to_string())
        .unwrap_or_default()
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/405-agent-react-multi-tool/vwfd/workflows", 3124)
        .native("react_tool_dispatcher", react_tool_dispatcher)
        .run().await;
}
