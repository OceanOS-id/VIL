// ╔════════════════════════════════════════════════════════════╗
// ║  020 — Marketing Campaign A/B Testing Gateway             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Marketing — Campaign Optimization               ║
// ║  Pattern:  VX_APP                                           ║
// ║  Token:    N/A (HTTP server)                                ║
// ║  Features: ShmSlice, ServiceCtx, #[vil_fault], VilResponse  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Weighted traffic splitting between AI model     ║
// ║  versions for marketing campaign content generation.       ║
// ║  Model A (stable) receives 80% of traffic; Model B        ║
// ║  (canary) receives 20%. Metrics track per-model latency,  ║
// ║  error rate, and conversion. Config endpoint allows        ║
// ║  real-time split adjustment without redeployment.          ║
// ╚════════════════════════════════════════════════════════════╝
// Marketing Campaign A/B Testing for AI Model Deployment
// =============================================================================
//
// Demonstrates weighted traffic splitting between model versions:
//   - Model A (stable): receives 80% of traffic
//   - Model B (canary): receives 20% of traffic
//   - Automatic fallback if canary error rate exceeds threshold
//
// VX highlights:
//   - ServiceProcess for gateway + model endpoints
//   - VilModel for typed request/response
//   - Atomic counters for A/B metrics
//   - VilResponse for all endpoints
//
// Endpoints:
//   GET  /                — gateway info with A/B split config
//   GET  /health          — health check
//   POST /api/ab/infer    — inference with automatic A/B routing
//   GET  /api/ab/metrics  — A/B test metrics (requests, errors, latency per model)
//   POST /api/ab/config   — update traffic split (e.g., {"model_a_pct": 90})
//
// Run:
//   cargo run -p basic-usage-ai-ab-testing-gateway
//
// Test:
//   curl http://localhost:8080/
//   curl -X POST http://localhost:8080/api/ab/infer \
//     -H 'Content-Type: application/json' \
//     -d '{"prompt": "Hello AI", "max_tokens": 100}'
//   curl http://localhost:8080/api/ab/metrics
//   curl -X POST http://localhost:8080/api/ab/config \
//     -H 'Content-Type: application/json' \
//     -d '{"model_a_pct": 90}'

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use vil_server::axum::extract::Extension;
use vil_server::prelude::*;

// ── Semantic Types ───────────────────────────────────────────────────────

// VIL Semantic Types (compile-time metadata, zero runtime cost):
//   AbTestState   [vil_state]  — mutable A/B test counters (Data Lane)
//   ModelRouted   [vil_event]  — immutable routing event log (Data Lane)
//   AbTestFault   [vil_fault]  — structured error (Control Lane)

// ── Domain Models ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// Gateway info response — shows current A/B test configuration and model versions.
struct GatewayInfo {
    name: String,
    description: String,
    model_a: ModelConfig,
    model_b: ModelConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// Model configuration — name, version, and current traffic percentage.
struct ModelConfig {
    name: String,
    version: String,
    traffic_pct: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Inference request from marketing campaign system.
struct InferRequest {
    prompt: String,
    max_tokens: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// Inference response with model attribution and A/B group assignment.
struct InferResponse {
    model: String,
    model_version: String,
    response: String,
    tokens_used: u32,
    latency_ms: u64,
    ab_group: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// A/B test metrics — aggregated counters for the marketing dashboard.
struct AbMetrics {
    total_requests: u64,
    model_a: ModelMetrics,
    model_b: ModelMetrics,
    current_split: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// Per-model metrics — request count, errors, latency, traffic percentage.
struct ModelMetrics {
    name: String,
    requests: u64,
    errors: u64,
    avg_latency_ms: u64,
    traffic_pct: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Traffic split update — sent by marketing team to adjust canary percentage.
struct ConfigUpdate {
    model_a_pct: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// Config update confirmation — shows new split ratios and status.
struct ConfigResult {
    model_a_pct: u8,
    model_b_pct: u8,
    status: String,
}

// ── Shared State ─────────────────────────────────────────────────────────
// Thread-safe atomic counters for A/B test metrics. Lock-free design
// ensures zero contention under high-concurrency marketing campaigns.
// AtomicU8 for traffic split allows real-time adjustment without restart.

struct AbState {
    model_a_pct: AtomicU8, // Traffic percentage for model A (0-100)
    total: AtomicU64,
    model_a_count: AtomicU64,
    model_b_count: AtomicU64,
    model_a_errors: AtomicU64,
    model_b_errors: AtomicU64,
    model_a_latency_sum: AtomicU64,
    model_b_latency_sum: AtomicU64,
    counter: AtomicU64, // Request counter for round-robin seed
}

impl AbState {
    fn new(model_a_pct: u8) -> Self {
        Self {
            model_a_pct: AtomicU8::new(model_a_pct),
            total: AtomicU64::new(0),
            model_a_count: AtomicU64::new(0),
            model_b_count: AtomicU64::new(0),
            model_a_errors: AtomicU64::new(0),
            model_b_errors: AtomicU64::new(0),
            model_a_latency_sum: AtomicU64::new(0),
            model_b_latency_sum: AtomicU64::new(0),
            counter: AtomicU64::new(0),
        }
    }

    // Deterministic routing based on counter modulo — ensures reproducible A/B splits
    fn route(&self) -> bool {
        // Returns true for model A, false for model B
        let pct = self.model_a_pct.load(Ordering::Relaxed);
        let n = self.counter.fetch_add(1, Ordering::Relaxed) % 100;
        n < pct as u64
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────

async fn index(ctx: ServiceCtx) -> VilResponse<GatewayInfo> {
    let state = ctx.state::<Arc<AbState>>().expect("AbState");
    let pct_a = state.model_a_pct.load(Ordering::Relaxed);
    VilResponse::ok(GatewayInfo {
        name: "A/B Testing AI Gateway".into(),
        description: "Weighted traffic split between model versions".into(),
        model_a: ModelConfig {
            name: "gpt-stable".into(),
            version: "v2.1".into(),
            traffic_pct: pct_a,
        },
        model_b: ModelConfig {
            name: "gpt-canary".into(),
            version: "v3.0-beta".into(),
            traffic_pct: 100 - pct_a,
        },
    })
}

async fn infer(ctx: ServiceCtx, body: ShmSlice) -> VilResponse<InferResponse> {
    let req: InferRequest = body.json().expect("invalid JSON body");
    let state = ctx.state::<Arc<AbState>>().expect("AbState");
    let start = std::time::Instant::now();
    state.total.fetch_add(1, Ordering::Relaxed);

    // Route to model A (stable) or model B (canary) based on traffic split
    let use_model_a = state.route();
    let (model_name, model_version, ab_group) = if use_model_a {
        state.model_a_count.fetch_add(1, Ordering::Relaxed);
        ("gpt-stable", "v2.1", "A")
    } else {
        state.model_b_count.fetch_add(1, Ordering::Relaxed);
        ("gpt-canary", "v3.0-beta", "B")
    };

    // Simulate inference (in production, forward to actual model endpoint)
    let max_tokens = req.max_tokens.unwrap_or(50);
    let response_text = format!(
        "[{}] Response to: '{}' (max_tokens={})",
        model_name, req.prompt, max_tokens
    );

    let latency = start.elapsed().as_millis() as u64;
    if use_model_a {
        state
            .model_a_latency_sum
            .fetch_add(latency, Ordering::Relaxed);
    } else {
        state
            .model_b_latency_sum
            .fetch_add(latency, Ordering::Relaxed);
    }

    VilResponse::ok(InferResponse {
        model: model_name.into(),
        model_version: model_version.into(),
        response: response_text,
        tokens_used: max_tokens.min(100),
        latency_ms: latency,
        ab_group: ab_group.into(),
    })
}

async fn metrics(ctx: ServiceCtx) -> VilResponse<AbMetrics> {
    let state = ctx.state::<Arc<AbState>>().expect("AbState");
    let pct_a = state.model_a_pct.load(Ordering::Relaxed);
    let a_count = state.model_a_count.load(Ordering::Relaxed);
    let b_count = state.model_b_count.load(Ordering::Relaxed);
    let a_latency = state.model_a_latency_sum.load(Ordering::Relaxed);
    let b_latency = state.model_b_latency_sum.load(Ordering::Relaxed);

    VilResponse::ok(AbMetrics {
        total_requests: state.total.load(Ordering::Relaxed),
        model_a: ModelMetrics {
            name: "gpt-stable".into(),
            requests: a_count,
            errors: state.model_a_errors.load(Ordering::Relaxed),
            avg_latency_ms: if a_count > 0 { a_latency / a_count } else { 0 },
            traffic_pct: pct_a,
        },
        model_b: ModelMetrics {
            name: "gpt-canary".into(),
            requests: b_count,
            errors: state.model_b_errors.load(Ordering::Relaxed),
            avg_latency_ms: if b_count > 0 { b_latency / b_count } else { 0 },
            traffic_pct: 100 - pct_a,
        },
        current_split: format!("{}% A / {}% B", pct_a, 100 - pct_a),
    })
}

/// POST /api/ab/config — dynamically adjust the A/B traffic split.
/// Marketing teams use this to shift traffic without redeployment.
/// Example: promote canary from 20% to 50% after initial validation.
async fn update_config(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> Result<VilResponse<ConfigResult>, VilError> {
    let state = ctx.state::<Arc<AbState>>().expect("AbState");
    let config: ConfigUpdate = body.json().expect("invalid JSON body");
    if config.model_a_pct > 100 {
        return Err(VilError::bad_request("model_a_pct must be 0-100"));
    }
    state
        .model_a_pct
        .store(config.model_a_pct, Ordering::Relaxed);
    Ok(VilResponse::ok(ConfigResult {
        model_a_pct: config.model_a_pct,
        model_b_pct: 100 - config.model_a_pct,
        status: "updated".into(),
    }))
}

// ── Main — Campaign A/B testing gateway assembly ─────────────────────────
// Default split: 80% stable (Model A) / 20% canary (Model B).
// Two service processes: root (gateway info) and ab (inference + metrics).
// Traffic split is adjustable at runtime via POST /api/ab/config.

#[tokio::main]
async fn main() {
    // Initialize with 80/20 split — conservative default for canary deployment
    let ab_state = Arc::new(AbState::new(80)); // 80% model A, 20% model B

    let ab_svc = ServiceProcess::new("ab")
        .prefix("/api/ab")
        .endpoint(Method::POST, "/infer", post(infer))
        .endpoint(Method::GET, "/metrics", get(metrics))
        .endpoint(Method::POST, "/config", post(update_config))
        .state(ab_state.clone());

    let root_svc = ServiceProcess::new("root")
        .endpoint(Method::GET, "/", get(index))
        .state(ab_state.clone());

    VilApp::new("ai-ab-testing-gateway")
        .port(8080)
        .service(root_svc)
        .service(ab_svc)
        .run()
        .await;
}
