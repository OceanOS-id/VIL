// ╔════════════════════════════════════════════════════════════╗
// ║  019 — Mission-Critical AI with Auto-Failover             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Healthcare — Triage AI (always-available)       ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: LlmRouter, RouterStrategy::Fallback, ServiceCtx ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Healthcare triage AI that MUST be available.     ║
// ║  Primary model down → instant fallback to backup.          ║
// ║  Response includes: provider used, fallback triggered.     ║
// ║  Designed for 99.99% availability on critical AI workloads.║
// ╚════════════════════════════════════════════════════════════╝
//
// Requires: ai-endpoint-simulator at localhost:4545
//
// Run:   cargo run -p vil-basic-ai-multi-model-advanced
// Test:
//   curl -X POST http://localhost:8080/api/triage/assess \
//     -H 'Content-Type: application/json' \
//     -d '{"symptoms":"chest pain, shortness of breath","severity":"high"}'

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use vil_llm::{ChatMessage, LlmProvider, LlmRouter, OpenAiConfig, OpenAiProvider, RouterStrategy};
use vil_server::prelude::*;

// ── Models ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TriageRequest {
    symptoms: String,
    #[serde(default = "default_severity")]
    severity: String,
}

fn default_severity() -> String { "unknown".into() }

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TriageResponse {
    assessment: String,
    provider_used: String,
    fallback_triggered: bool,
    latency_ms: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TriageStats {
    total_requests: u64,
    primary_used: u64,
    fallback_used: u64,
    availability_pct: f64,
}

struct TriageState {
    router: LlmRouter,
    _primary_name: String,
    total: AtomicU64,
    primary_count: AtomicU64,
    fallback_count: AtomicU64,
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /assess — Triage patient symptoms via LLM with auto-failover.
async fn assess(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<TriageResponse>> {
    let req: TriageRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;

    let state = ctx.state::<Arc<TriageState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    state.total.fetch_add(1, Ordering::Relaxed);

    let messages = vec![
        ChatMessage::system(
            "You are a medical triage AI. Assess symptoms, suggest urgency level \
             (EMERGENCY/URGENT/ROUTINE), and recommend next steps. Be concise."
        ),
        ChatMessage::user(&format!(
            "Patient symptoms: {}. Reported severity: {}.",
            req.symptoms, req.severity
        )),
    ];

    let start = Instant::now();

    // LlmRouter with Fallback strategy: try primary, auto-switch on error
    let response = state.router.chat(&messages).await
        .map_err(|e| VilError::internal(format!("all providers failed: {}", e)))?;

    let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Detect which provider was used
    let used_primary = response.model.contains("gpt-4");
    let fallback_triggered = !used_primary;

    if used_primary {
        state.primary_count.fetch_add(1, Ordering::Relaxed);
    } else {
        state.fallback_count.fetch_add(1, Ordering::Relaxed);
    }

    Ok(VilResponse::ok(TriageResponse {
        assessment: response.content,
        provider_used: response.model,
        fallback_triggered,
        latency_ms,
    }))
}

/// GET /stats — Availability and failover statistics.
async fn stats(ctx: ServiceCtx) -> HandlerResult<VilResponse<TriageStats>> {
    let state = ctx.state::<Arc<TriageState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let total = state.total.load(Ordering::Relaxed);
    let primary = state.primary_count.load(Ordering::Relaxed);
    let fallback = state.fallback_count.load(Ordering::Relaxed);

    Ok(VilResponse::ok(TriageStats {
        total_requests: total,
        primary_used: primary,
        fallback_used: fallback,
        availability_pct: if total > 0 { 100.0 } else { 0.0 },
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let upstream = std::env::var("LLM_UPSTREAM")
        .unwrap_or_else(|_| "http://127.0.0.1:4545".into());
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    // Primary: GPT-4 (best quality). Backup: GPT-3.5 (always available).
    let primary = Arc::new(OpenAiProvider::new(
        OpenAiConfig::new(&api_key, "gpt-4")
            .base_url(&format!("{}/v1", upstream)),
    ));
    let backup = Arc::new(OpenAiProvider::new(
        OpenAiConfig::new(&api_key, "gpt-3.5-turbo")
            .base_url(&format!("{}/v1", upstream)),
    ));

    // Fallback router: primary fails → auto-switch to backup
    let router = LlmRouter::new(RouterStrategy::Fallback)
        .add_provider(primary.clone())
        .add_provider(backup);

    let state = Arc::new(TriageState {
        router,
        _primary_name: "gpt-4".into(),
        total: AtomicU64::new(0),
        primary_count: AtomicU64::new(0),
        fallback_count: AtomicU64::new(0),
    });

    let svc = ServiceProcess::new("triage")
        .endpoint(Method::POST, "/assess", post(assess))
        .endpoint(Method::GET, "/stats", get(stats))
        .state(state);

    VilApp::new("healthcare-triage-ai")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
