// 034 — Credit Risk Monte Carlo (Hybrid: Sidecar Python for Monte Carlo sim, NativeCode for health)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/034-basic-blocking-task/vwfd/workflows", 8080)
        // Health check — NativeCode (trivial status)
        .native("risk_health_handler", |_| {
            Ok(json!({"status": "healthy", "service": "risk-engine", "model": "monte-carlo-v2"}))
        })
        // Risk assessment — Sidecar Python (Monte Carlo simulation, CPU-bound blocking)
        .sidecar("monte_carlo_risk", "python3 examples/034-basic-blocking-task/vwfd/sidecar/python/monte_carlo_risk.py")
        .run()
        .await;
}
