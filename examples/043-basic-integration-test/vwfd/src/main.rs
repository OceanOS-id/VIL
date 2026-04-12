// 043-basic-integration-test — VWFD mode
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/043-basic-integration-test/vwfd/workflows", 8080)
        .native("list_tasks_handler", |_| {
            Ok(json!({"data": [], "total": 0}))
        })
        .native("create_task_handler", |input| {
            let body = &input["body"];
            Ok(json!({"id": 1, "title": body["title"], "done": false, "created": true}))
        })
        .native("task_stats_handler", |_| {
            Ok(json!({"total": 0, "done": 0, "pending": 0}))
        })
        .run()
        .await;
}
