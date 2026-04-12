// 017 — Enterprise Sprint Tracker (NativeCode — stateful in-memory CRUD + auth)
use serde_json::{json, Value};
use std::sync::Mutex;
use std::sync::OnceLock;

static SPRINTS: OnceLock<Mutex<Vec<Value>>> = OnceLock::new();
fn sprints() -> &'static Mutex<Vec<Value>> {
    SPRINTS.get_or_init(|| Mutex::new(vec![
        json!({"id": 1, "title": "Sprint Alpha", "status": "in_progress", "assignee": "alice", "story_points": 8}),
        json!({"id": 2, "title": "Sprint Beta", "status": "planned", "assignee": "bob", "story_points": 5}),
        json!({"id": 3, "title": "Sprint Gamma", "status": "done", "assignee": "carol", "story_points": 13}),
    ]))
}
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(4);

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/017-basic-production-fullstack/vwfd/workflows", 8080)
        .native("platform_config", |_| {
            Ok(json!({
                "server_name": "vil-sprint-tracker",
                "version": "1.0.0",
                "observer_enabled": true,
                "max_sprints": 1000,
            }))
        })
        .native("sprint_list", |_| {
            let store = sprints().lock().unwrap();
            Ok(json!(store.clone()))
        })
        .native("sprint_stats", |_| {
            let store = sprints().lock().unwrap();
            let total = store.len();
            let done = store.iter().filter(|s| s["status"] == "done").count();
            let total_sp: i64 = store.iter().filter_map(|s| s["story_points"].as_i64()).sum();
            let done_sp: i64 = store.iter().filter(|s| s["status"] == "done")
                .filter_map(|s| s["story_points"].as_i64()).sum();
            let velocity_pct = if total_sp > 0 { (done_sp as f64 / total_sp as f64 * 100.0).round() } else { 0.0 };
            Ok(json!({
                "total": total, "done": done, "in_progress": total - done,
                "total_story_points": total_sp,
                "completed_story_points": done_sp,
                "velocity_pct": velocity_pct,
            }))
        })
        .native("sprint_create", |input| {
            let body = &input["body"];
            let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let sprint = json!({
                "id": id,
                "title": body["title"].as_str().unwrap_or("Untitled"),
                "status": body["status"].as_str().unwrap_or("planned"),
                "assignee": body["assignee"].as_str().unwrap_or("unassigned"),
                "story_points": body["story_points"].as_i64().unwrap_or(0),
            });
            sprints().lock().unwrap().push(sprint.clone());
            Ok(sprint)
        })
        .native("sprint_update", |input| {
            let body = &input["body"];
            let id = body["id"].as_i64().unwrap_or(0);
            let mut store = sprints().lock().unwrap();
            if let Some(sprint) = store.iter_mut().find(|s| s["id"] == id) {
                if let Some(st) = body["status"].as_str() { sprint["status"] = json!(st); }
                if let Some(a) = body["assignee"].as_str() { sprint["assignee"] = json!(a); }
                Ok(sprint.clone())
            } else {
                Err(format!("404:Sprint {} not found", id))
            }
        })
        .run().await;
}
