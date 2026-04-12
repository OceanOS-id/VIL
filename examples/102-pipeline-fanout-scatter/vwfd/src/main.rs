// 102 — NPL Fan-Out Filter (Go WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/102-pipeline-fanout-scatter/vwfd/workflows", 3302)
        .wasm("fanout_npl_filter", "examples/102-pipeline-fanout-scatter/vwfd/wasm/go/fanout_npl.wasm")
        .run().await;
}
