// 705 — Payment Gateway (Java WASM)
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/705-protocol-grpc-gateway/vwfd/workflows", 3705)
        .wasm("payment_validate_card", "examples/705-protocol-grpc-gateway/vwfd/wasm/java/PaymentProcessor.class")
        .wasm("payment_process_charge", "examples/705-protocol-grpc-gateway/vwfd/wasm/java/PaymentProcessor.class")
        .run().await;
}
