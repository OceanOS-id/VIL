// 107 — Supply Chain Traced (Go WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/107-pipeline-process-traced/vwfd/workflows", 3307)
        .wasm("traced_supply_chain", "examples/107-pipeline-process-traced/vwfd/wasm/go/supply_chain.wasm")
        .run().await;
}
