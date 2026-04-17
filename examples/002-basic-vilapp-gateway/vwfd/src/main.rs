// 002-basic-vilapp-gateway — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/002-basic-vilapp-gateway/vwfd/workflows", 3081)
        .run()
        .await;
}
