// 041 — ML Scoring with HA Failover + Circuit Breaker (NativeCode)
// Business logic matches standard src/main.rs:
//   - Primary: weighted sigmoid scoring
//   - Backup: simple average fallback
//   - Circuit breaker: CLOSED(0) → OPEN(1) → HALF_OPEN(2) on 3 failures
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};

static CIRCUIT_STATE: AtomicU64 = AtomicU64::new(0); // 0=closed, 1=open, 2=half-open
static PRIMARY_FAILURES: AtomicU64 = AtomicU64::new(0);
static FAILOVER_COUNT: AtomicU64 = AtomicU64::new(0);
static TOTAL_REQUESTS: AtomicU64 = AtomicU64::new(0);
const FAILURE_THRESHOLD: u64 = 3;

fn score_primary(features: &[f64]) -> f64 {
    let mut sum = 0.0;
    for (i, f) in features.iter().enumerate() {
        let weight = 1.0 / (1.0 + i as f64 * 0.3);
        sum += f * weight;
    }
    1.0 / (1.0 + (-sum).exp()) // sigmoid
}

fn score_backup(features: &[f64]) -> f64 {
    if features.is_empty() { return 0.5; }
    features.iter().sum::<f64>() / features.len() as f64
}

fn circuit_state_name(state: u64) -> &'static str {
    match state { 0 => "CLOSED", 1 => "OPEN", 2 => "HALF_OPEN", _ => "UNKNOWN" }
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/041-basic-sidecar-failover/vwfd/workflows", 8080)
        .native("predict_handler", |input| {
            let body = &input["body"];
            let features: Vec<f64> = body["features"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                .unwrap_or_default();

            TOTAL_REQUESTS.fetch_add(1, Ordering::Relaxed);
            let state = CIRCUIT_STATE.load(Ordering::Relaxed);

            let (prediction, confidence, served_by, model_version, failover_used);

            if state == 1 { // OPEN — mostly backup, 10% probe
                let total = TOTAL_REQUESTS.load(Ordering::Relaxed);
                if total % 10 == 0 {
                    CIRCUIT_STATE.store(2, Ordering::Relaxed); // try half-open
                    let p = score_primary(&features);
                    if p >= 0.0 {
                        PRIMARY_FAILURES.store(0, Ordering::Relaxed);
                        CIRCUIT_STATE.store(0, Ordering::Relaxed); // close circuit
                        prediction = p;
                        confidence = 0.85 + (p * 0.15_f64).min(0.14);
                        served_by = "primary";
                        model_version = "v2.1";
                        failover_used = false;
                    } else {
                        prediction = score_backup(&features);
                        confidence = 0.70 + (prediction * 0.10_f64).min(0.09);
                        served_by = "backup";
                        model_version = "v1.0";
                        failover_used = true;
                        FAILOVER_COUNT.fetch_add(1, Ordering::Relaxed);
                    }
                } else {
                    prediction = score_backup(&features);
                    confidence = 0.70 + (prediction * 0.10_f64).min(0.09);
                    served_by = "backup";
                    model_version = "v1.0";
                    failover_used = true;
                    FAILOVER_COUNT.fetch_add(1, Ordering::Relaxed);
                }
            } else { // CLOSED or HALF_OPEN — try primary
                let p = score_primary(&features);
                if p >= 0.0 {
                    if state == 2 { // was half-open, recovery successful
                        PRIMARY_FAILURES.store(0, Ordering::Relaxed);
                        CIRCUIT_STATE.store(0, Ordering::Relaxed);
                    }
                    prediction = p;
                    confidence = 0.85 + (p * 0.15_f64).min(0.14);
                    served_by = "primary";
                    model_version = "v2.1";
                    failover_used = false;
                } else {
                    let fails = PRIMARY_FAILURES.fetch_add(1, Ordering::Relaxed) + 1;
                    if fails >= FAILURE_THRESHOLD {
                        CIRCUIT_STATE.store(1, Ordering::Relaxed);
                    }
                    prediction = score_backup(&features);
                    confidence = 0.70 + (prediction * 0.10_f64).min(0.09);
                    served_by = "backup";
                    model_version = "v1.0";
                    failover_used = true;
                    FAILOVER_COUNT.fetch_add(1, Ordering::Relaxed);
                }
            }

            Ok(json!({
                "prediction": prediction,
                "confidence": confidence,
                "model_version": model_version,
                "served_by": served_by,
                "failover_used": failover_used
            }))
        })
        .native("health_handler", |_| {
            let state = CIRCUIT_STATE.load(Ordering::Relaxed);
            let primary_status = if state == 1 { "degraded" } else { "healthy" };
            Ok(json!({
                "primary_status": primary_status,
                "backup_status": "healthy",
                "circuit_state": circuit_state_name(state),
                "failover_count": FAILOVER_COUNT.load(Ordering::Relaxed),
                "primary_failures": PRIMARY_FAILURES.load(Ordering::Relaxed),
                "total_requests": TOTAL_REQUESTS.load(Ordering::Relaxed)
            }))
        })
        .run()
        .await;
}
