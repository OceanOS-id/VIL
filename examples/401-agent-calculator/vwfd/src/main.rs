// 401-agent-calculator — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/401-agent-calculator/vwfd/workflows", 3120)
        .run()
        .await;
}
