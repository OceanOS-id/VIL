// 603-db-clickhouse-batch — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/603-db-clickhouse-batch/vwfd/workflows", 3243)
        .run()
        .await;
}
