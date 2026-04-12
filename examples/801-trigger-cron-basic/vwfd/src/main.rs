// 801-trigger-cron-basic — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/801-trigger-cron-basic/vwfd/workflows", 3258)
        .run()
        .await;
}
