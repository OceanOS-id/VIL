// 805-trigger-email — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/805-trigger-email/vwfd/workflows", 3262)
        .run()
        .await;
}
