// 803-trigger-webhook-receiver — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/803-trigger-webhook-receiver/vwfd/workflows", 3260)
        .run()
        .await;
}
