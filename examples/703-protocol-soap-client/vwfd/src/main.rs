// 703-protocol-soap-client — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/703-protocol-soap-client/vwfd/workflows", 3254)
        .run()
        .await;
}
