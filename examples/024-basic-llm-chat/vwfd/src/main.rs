// 024-basic-llm-chat — LLM Customer Support Chatbot (VWFD)
//
// Endpoints:
//   POST /api/chat → chat response (returns "content")

use serde_json::{json, Value};

fn chat_handler(input: &Value) -> Result<Value, String> {
    let prompt = input.get("body")
        .and_then(|b| b["prompt"].as_str())
        .unwrap_or("Hello");
    Ok(json!({
        "content": format!("I'd be happy to help you with: {}. As a customer support assistant, I can answer questions about our products, services, and policies.", prompt),
        "model": "gpt-4",
        "tokens_used": 42,
        "finish_reason": "stop"
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/024-basic-llm-chat/vwfd/workflows", 3090)
        .native("chat_handler", chat_handler)
        .run()
        .await;
}
