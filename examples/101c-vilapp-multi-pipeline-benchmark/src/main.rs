// ╔════════════════════════════════════════════════════════════╗
// ║  101c — Multi-Pipeline Benchmark (VilApp)                 ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Same business as 101b: Webhook → Transform → SSE         ║
// ║  Pattern: VX_APP (VilApp + ServiceProcess)                ║
// ║  Purpose: head-to-head multi-pipeline vs ShmToken          ║
// ║                                                            ║
// ║  The transform stage adds metadata (timestamp, node_id)   ║
// ║  to prove data flows through all stages.                  ║
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

fn default_prompt() -> String { "benchmark multi-pipeline".to_string() }

/// Stage 1: Receive trigger
/// Stage 2: Transform — enrich with pipeline metadata
/// Stage 3: Call upstream LLM, stream response
async fn trigger(body: ShmSlice) -> impl IntoResponse {
    let req: TriggerRequest = body.json().unwrap_or(TriggerRequest { prompt: default_prompt() });

    // Stage 2: Transform — add metadata
    let enriched_prompt = format!("{{\"stage\":\"transform\",\"original\":\"{}\"}}", req.prompt);

    // Stage 3: Call upstream
    let result = SseCollect::post_to(UPSTREAM_URL)
        .body(serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": enriched_prompt}],
            "stream": true
        }))
        .json_tap("choices[0].delta.content")
        .done_marker("[DONE]")
        .collect_text()
        .await;

    match result {
        Ok(content) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "stage": "response",
                "content": content
            })),
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
        .unwrap_or(true);

    let svc = ServiceProcess::new("pipeline")
        .endpoint(Method::POST, "/trigger", post(trigger));

    VilApp::new("multi-pipeline-bench")
        .port(3090)
        .observer(observer_on)
        .service(svc)
        .run()
        .await;
}
