// 005 — Payload Validator (Rust WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/005-basic-multiservice-mesh-ndjson/vwfd/workflows", 3105)
        .wasm("validate_payload", "examples/005-basic-multiservice-mesh-ndjson/vwfd/wasm/rust/validate_payload.wasm")
        .run().await;
}
