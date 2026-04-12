// 020 — A/B Testing Gateway (Hybrid: WASM Rust for deterministic split, NativeCode for metrics/config)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/020-basic-ai-ab-testing/vwfd/workflows", 8080)
        // Deterministic A/B split — WASM Rust (hash-based, sandboxed)
        .wasm("ab_infer_handler", "examples/020-basic-ai-ab-testing/vwfd/wasm/rust/deterministic_split.wasm")
        // Metrics endpoint — NativeCode (static mock data)
        .native("ab_metrics_handler", |_| {
            Ok(json!({
                "total_requests": 1024,
                "current_split": {"A": 80, "B": 20},
                "model_a_latency_ms": 45,
                "model_b_latency_ms": 52
            }))
        })
        // Config update — NativeCode (simple parameter echo)
        .native("ab_config_handler", |input| {
            let body = input.get("input").and_then(|i| i.get("body")).unwrap_or(input);
            let pct = body.get("model_a_pct").and_then(|v| v.as_u64()).unwrap_or(80);
            Ok(json!({
                "updated": true,
                "model_a_pct": pct,
                "model_b_pct": 100 - pct
            }))
        })
        .run()
        .await;
}
