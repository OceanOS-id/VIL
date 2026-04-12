// 041 — ML Scoring with Sidecar Failover (Python sidecar, zero NativeCode)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/041-basic-sidecar-failover/vwfd/workflows", 8080)
        .sidecar("ml_scorer", "python3 examples/041-basic-sidecar-failover/vwfd/sidecar/python/ml_scorer.py")
        .run()
        .await;
}
