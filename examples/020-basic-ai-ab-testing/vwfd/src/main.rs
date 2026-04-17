// 020 — A/B Testing AI Gateway (VWFD)
// Business logic identical to standard:
//   - Deterministic routing: counter % 100 < model_a_pct → model A
//   - Atomic counters: total, model_a_count, model_b_count, latency sums
//   - Models: gpt-stable v2.1 (A) vs gpt-canary v3.0-beta (B)
//   - Config: dynamic split adjustment via POST /api/ab/config
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

static MODEL_A_PCT: AtomicU8 = AtomicU8::new(70);
static TOTAL: AtomicU64 = AtomicU64::new(0);
static MODEL_A_COUNT: AtomicU64 = AtomicU64::new(0);
static MODEL_B_COUNT: AtomicU64 = AtomicU64::new(0);
static MODEL_A_LATENCY_SUM: AtomicU64 = AtomicU64::new(0);
static MODEL_B_LATENCY_SUM: AtomicU64 = AtomicU64::new(0);
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn route_model() -> bool {
    let pct = MODEL_A_PCT.load(Ordering::Relaxed);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed) % 100;
    n < pct as u64
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/020-basic-ai-ab-testing/vwfd/workflows", 8080)
        .native("ab_infer_handler", |input| {
            let start = std::time::Instant::now();
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let prompt = body["prompt"].as_str().unwrap_or("");
            let max_tokens = body["max_tokens"].as_u64().unwrap_or(50);

            TOTAL.fetch_add(1, Ordering::Relaxed);
            let use_a = route_model();
            let (model, version, group) = if use_a {
                MODEL_A_COUNT.fetch_add(1, Ordering::Relaxed);
                ("gpt-stable", "v2.1", "A")
            } else {
                MODEL_B_COUNT.fetch_add(1, Ordering::Relaxed);
                ("gpt-canary", "v3.0-beta", "B")
            };

            let response = format!("[{}] Response to: '{}' (max_tokens={})", model, prompt, max_tokens);
            let latency = start.elapsed().as_millis() as u64;
            if use_a {
                MODEL_A_LATENCY_SUM.fetch_add(latency, Ordering::Relaxed);
            } else {
                MODEL_B_LATENCY_SUM.fetch_add(latency, Ordering::Relaxed);
            }

            Ok(json!({
                "model": model, "model_version": version,
                "response": response, "tokens_used": max_tokens.min(100),
                "latency_ms": latency, "ab_group": group
            }))
        })
        .native("ab_metrics_handler", |_| {
            let pct_a = MODEL_A_PCT.load(Ordering::Relaxed);
            let a_count = MODEL_A_COUNT.load(Ordering::Relaxed);
            let b_count = MODEL_B_COUNT.load(Ordering::Relaxed);
            let a_lat = MODEL_A_LATENCY_SUM.load(Ordering::Relaxed);
            let b_lat = MODEL_B_LATENCY_SUM.load(Ordering::Relaxed);
            Ok(json!({
                "total_requests": TOTAL.load(Ordering::Relaxed),
                "model_a": {
                    "name": "gpt-stable", "requests": a_count,
                    "errors": 0, "traffic_pct": pct_a,
                    "avg_latency_ms": if a_count > 0 { a_lat / a_count } else { 0 }
                },
                "model_b": {
                    "name": "gpt-canary", "requests": b_count,
                    "errors": 0, "traffic_pct": 100 - pct_a,
                    "avg_latency_ms": if b_count > 0 { b_lat / b_count } else { 0 }
                },
                "current_split": format!("{}% A / {}% B", pct_a, 100 - pct_a)
            }))
        })
        .native("ab_config_handler", |input| {
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let new_pct = body["model_a_pct"].as_u64().unwrap_or(70) as u8;
            let clamped = new_pct.min(100);
            MODEL_A_PCT.store(clamped, Ordering::Relaxed);
            Ok(json!({
                "model_a_pct": clamped,
                "model_b_pct": 100 - clamped,
                "status": "updated"
            }))
        })
        .run().await;
}
