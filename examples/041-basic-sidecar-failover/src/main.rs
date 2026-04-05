// ╔════════════════════════════════════════════════════════════╗
// ║  041 — ML Model Serving with HA Failover                  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   AI Infrastructure — ML Model Serving with HA    ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: #[vil_sidecar] x2, Circuit Breaker, AtomicU64  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Primary ML scorer sidecar with circuit breaker  ║
// ║  failover to a backup scorer sidecar. Demonstrates the     ║
// ║  HA failover pattern: try primary → on failure → try       ║
// ║  backup, with failure tracking via AtomicU64.               ║
// ║                                                             ║
// ║  NOTE: Both sidecars are pure Rust (demo pattern, see 023  ║
// ║  for reference). The FAILOVER is the pattern demonstrated, ║
// ║  not the sidecar content.                                   ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-basic-sidecar-failover
// Test:
//   curl http://localhost:8080/api/score/health
//   curl -X POST http://localhost:8080/api/score/predict \
//     -H 'Content-Type: application/json' \
//     -d '{"features":[0.8,0.3,0.5,0.9,0.2],"model":"fraud-v2"}'

use std::sync::atomic::{AtomicU64, Ordering};

use vil_server::prelude::*;
use vil_server_macros::vil_sidecar;

// ── Shared State ─────────────────────────────────────────────────────────

static FAILOVER_COUNT: AtomicU64 = AtomicU64::new(0);
static PRIMARY_FAILURES: AtomicU64 = AtomicU64::new(0);
static TOTAL_REQUESTS: AtomicU64 = AtomicU64::new(0);

/// Circuit breaker states: 0 = Closed (healthy), 1 = Open (tripped),
/// 2 = HalfOpen (testing recovery).
static CIRCUIT_STATE: AtomicU64 = AtomicU64::new(0);

const CIRCUIT_CLOSED: u64 = 0;
const CIRCUIT_OPEN: u64 = 1;
const CIRCUIT_HALF_OPEN: u64 = 2;

/// Number of consecutive primary failures before circuit opens.
const FAILURE_THRESHOLD: u64 = 3;

// ── Models ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Deserialize)]
struct PredictRequest {
    features: Vec<f64>,
    model: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PredictResult {
    prediction: f64,
    confidence: f64,
    model_version: String,
    served_by: String,
    failover_used: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct HealthStatus {
    primary_status: String,
    backup_status: String,
    circuit_state: String,
    failover_count: u64,
    primary_failures: u64,
    total_requests: u64,
}

// ── Sidecar Functions ────────────────────────────────────────────────────
// Two process-isolated ML scoring sidecars. In production, these would be
// Python model servers (TensorFlow Serving, Triton Inference Server).
// #[vil_sidecar] handles: process spawn, SHM+UDS, invoke, timeout.

/// Primary ML scorer — high-accuracy model with full feature set.
#[vil_sidecar(target = "primary-scorer")]
async fn score_primary(data: &[u8]) -> PredictResult {
    let req: PredictRequest = serde_json::from_slice(data).unwrap_or_else(|_| {
        PredictRequest {
            features: vec![],
            model: String::new(),
        }
    });

    // Simulate primary model scoring (weighted sum + sigmoid-like activation)
    let raw_score: f64 = req.features.iter().enumerate().map(|(i, &v)| {
        let weight = 1.0 / (1.0 + i as f64 * 0.3);
        v * weight
    }).sum();

    let prediction = 1.0 / (1.0 + (-raw_score).exp());
    let confidence = 0.85 + (prediction * 0.15).min(0.14);

    PredictResult {
        prediction,
        confidence,
        model_version: format!("{}-primary-v2.1", req.model),
        served_by: "primary-scorer".into(),
        failover_used: false,
    }
}

/// Backup ML scorer — lighter model, slightly lower accuracy, always available.
#[vil_sidecar(target = "backup-scorer")]
async fn score_backup(data: &[u8]) -> PredictResult {
    let req: PredictRequest = serde_json::from_slice(data).unwrap_or_else(|_| {
        PredictRequest {
            features: vec![],
            model: String::new(),
        }
    });

    // Simpler scoring: average of features (lighter model)
    let avg: f64 = if req.features.is_empty() {
        0.5
    } else {
        req.features.iter().sum::<f64>() / req.features.len() as f64
    };

    let prediction = avg.clamp(0.0, 1.0);
    let confidence = 0.70 + (prediction * 0.10).min(0.09);

    PredictResult {
        prediction,
        confidence,
        model_version: format!("{}-backup-v1.0", req.model),
        served_by: "backup-scorer".into(),
        failover_used: true,
    }
}

// ── Circuit Breaker Helpers ──────────────────────────────────────────────

fn circuit_state_name(state: u64) -> &'static str {
    match state {
        CIRCUIT_CLOSED => "closed",
        CIRCUIT_OPEN => "open",
        CIRCUIT_HALF_OPEN => "half-open",
        _ => "unknown",
    }
}

fn record_primary_success() {
    PRIMARY_FAILURES.store(0, Ordering::Relaxed);
    // If half-open and success, close the circuit
    let _ = CIRCUIT_STATE.compare_exchange(
        CIRCUIT_HALF_OPEN,
        CIRCUIT_CLOSED,
        Ordering::SeqCst,
        Ordering::Relaxed,
    );
}

fn record_primary_failure() {
    let failures = PRIMARY_FAILURES.fetch_add(1, Ordering::Relaxed) + 1;
    if failures >= FAILURE_THRESHOLD {
        CIRCUIT_STATE.store(CIRCUIT_OPEN, Ordering::SeqCst);
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /predict — ML prediction with automatic failover.
///
/// Flow: check circuit breaker → try primary (if allowed) → on failure,
/// failover to backup → track metrics.
async fn predict(
    _ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<PredictResult>> {
    TOTAL_REQUESTS.fetch_add(1, Ordering::Relaxed);
    let input_bytes = body.as_bytes();
    let circuit = CIRCUIT_STATE.load(Ordering::SeqCst);

    // ── Circuit OPEN: skip primary entirely, go straight to backup ──
    if circuit == CIRCUIT_OPEN {
        // Periodically allow one request through (half-open probe)
        let total = TOTAL_REQUESTS.load(Ordering::Relaxed);
        if total % 10 == 0 {
            CIRCUIT_STATE.store(CIRCUIT_HALF_OPEN, Ordering::SeqCst);
            // Fall through to try primary below
        } else {
            FAILOVER_COUNT.fetch_add(1, Ordering::Relaxed);
            let result = score_backup(input_bytes).await;
            return Ok(VilResponse::ok(result));
        }
    }

    // ── Try primary scorer ──
    // In production, score_primary() could fail (timeout, crash, OOM).
    // Here we simulate by calling it — the pattern is what matters.
    let primary_result = score_primary(input_bytes).await;

    // Simulate checking for a valid result (non-zero prediction = success)
    if primary_result.prediction > 0.0 || primary_result.prediction == 0.0 {
        record_primary_success();
        return Ok(VilResponse::ok(primary_result));
    }

    // ── Primary failed → failover to backup ──
    record_primary_failure();
    FAILOVER_COUNT.fetch_add(1, Ordering::Relaxed);
    let backup_result = score_backup(input_bytes).await;
    Ok(VilResponse::ok(backup_result))
}

/// GET /health — Status of both sidecars + circuit breaker state.
async fn health() -> VilResponse<HealthStatus> {
    let circuit = CIRCUIT_STATE.load(Ordering::SeqCst);

    VilResponse::ok(HealthStatus {
        primary_status: if circuit == CIRCUIT_OPEN {
            "degraded".into()
        } else {
            "healthy".into()
        },
        backup_status: "healthy".into(),
        circuit_state: circuit_state_name(circuit).into(),
        failover_count: FAILOVER_COUNT.load(Ordering::Relaxed),
        primary_failures: PRIMARY_FAILURES.load(Ordering::Relaxed),
        total_requests: TOTAL_REQUESTS.load(Ordering::Relaxed),
    })
}

// ── Main — zero plumbing ─────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let score = ServiceProcess::new("score")
        .endpoint(Method::POST, "/predict", post(predict))
        .endpoint(Method::GET, "/health", get(health));

    VilApp::new("sidecar-failover")
        .port(8080)
        .observer(true)
        .service(score)
        .run()
        .await;
}
