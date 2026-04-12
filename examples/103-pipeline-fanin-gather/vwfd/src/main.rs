// 103 — Fan-In Gather (Go Sidecar)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/103-pipeline-fanin-gather/vwfd/workflows", 3303)
        .sidecar("fanin_credit_inventory", "go run examples/103-pipeline-fanin-gather/vwfd/sidecar/go/fanin_aggregator.go")
        .run().await;
}
