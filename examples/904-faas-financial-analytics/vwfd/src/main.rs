// 904 — Financial Analytics (Workflow Pattern)
// Demonstrates: datefmt, duration, anomaly, geodist, stats in V-CEL
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/904-faas-financial-analytics/vwfd/workflows", 8080)
        .run()
        .await;
}
