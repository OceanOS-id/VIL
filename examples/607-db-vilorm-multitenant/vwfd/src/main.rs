// 607-db-vilorm-multitenant — VWFD mode (pure workflow, no NativeCode handlers needed)

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/607-db-vilorm-multitenant/vwfd/workflows", 8087)
        .run()
        .await;
}
