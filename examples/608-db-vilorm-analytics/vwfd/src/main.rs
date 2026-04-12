// 608-db-vilorm-analytics — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/608-db-vilorm-analytics/vwfd/workflows", 8088)
        .run()
        .await;
}
