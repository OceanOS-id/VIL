use serde_json::{json, Value};
fn react_tool_dispatcher(input: &Value) -> Result<Value, String> {
    let action = input["action"].as_str().unwrap_or("");
    let action_input = input["action_input"].as_str().unwrap_or("");
    match action {
        "search" => Ok(json!({"observation": format!("Found results for: {}", action_input), "tool": "search"})),
        "calculator" => {
            let parts: Vec<&str> = action_input.splitn(3, ' ').collect();
            if parts.len() == 3 {
                let (a, b) = (parts[0].parse::<f64>().unwrap_or(0.0), parts[2].parse::<f64>().unwrap_or(0.0));
                let r = match parts[1] { "+" => a+b, "-" => a-b, "*" => a*b, "/" => if b!=0.0 {a/b} else {0.0}, _ => 0.0 };
                Ok(json!({"observation": format!("{}", r), "tool": "calculator"}))
            } else { Ok(json!({"observation": action_input, "tool": "calculator"})) }
        }
        _ => Err(format!("unknown action: {}", action)),
    }
}
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/405-agent-react-multi-tool/vwfd/workflows", 3124)
        .native("react_tool_dispatcher", react_tool_dispatcher).run().await;
}
