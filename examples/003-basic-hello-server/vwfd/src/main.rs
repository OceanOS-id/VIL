// 003 — Currency Converter (Rust WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/003-basic-hello-server/vwfd/workflows", 8080)
        .wasm("currency_convert", "examples/003-basic-hello-server/vwfd/wasm/rust/currency_convert.wasm")
        .run().await;
}
