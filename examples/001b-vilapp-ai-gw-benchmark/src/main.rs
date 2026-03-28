// ╔════════════════════════════════════════════════════════════╗
// ║  001b — AI Gateway Benchmark (VilApp version)             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Same business as 001: SSE proxy to upstream LLM at 4545  ║
// ║  Pattern: VX_APP (VilApp + ServiceProcess)                ║
// ║  Purpose: head-to-head observer ON/OFF benchmark          ║
// ║                                                            ║
// ║  Toggle observer via env: OBSERVER=1 or OBSERVER=0        ║
// ╚════════════════════════════════════════════════════════════╝

use vil_server::prelude::*;

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

#[derive(Deserialize)]
struct TriggerRequest {
    #[serde(default = "default_prompt")]
    prompt: String,
}

fn default_prompt() -> String {
    "Hello".to_string()
}

async fn trigger(body: ShmSlice) -> impl IntoResponse {
    let req: TriggerRequest = body.json().unwrap_or(TriggerRequest {
        prompt: default_prompt(),
    });

    let result = SseCollect::post_to(UPSTREAM_URL)
        .body(serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": req.prompt}],
            "stream": true
        }))
        .json_tap("choices[0].delta.content")
        .done_marker("[DONE]")
        .collect_text()
        .await;

    match result {
        Ok(content) => (
            StatusCode::OK,
            Json(serde_json::json!({ "content": content })),
        ),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

#[tokio::main]
async fn main() {
    let observer_on = std::env::var("OBSERVER")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);

    let svc = ServiceProcess::new("gw").endpoint(Method::POST, "/trigger", post(trigger));

    let app = VilApp::new("ai-gw-bench")
        .port(3081)
        .observer(observer_on)
        .service(svc);

    app.run().await;
}
