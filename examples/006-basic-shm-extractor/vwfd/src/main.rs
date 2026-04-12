// 006 — Trade Data Processor (Go WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/006-basic-shm-extractor/vwfd/workflows", 3106)
        .wasm("process_trade_data", "examples/006-basic-shm-extractor/vwfd/wasm/go/process_trade.wasm")
        .run().await;
}
