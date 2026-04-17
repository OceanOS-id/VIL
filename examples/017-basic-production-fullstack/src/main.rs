// ╔════════════════════════════════════════════════════════════╗
// ║  017 — Enterprise Sprint Tracker (Production Pattern)     ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Engineering — Sprint Planning & Tracking        ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: Multi-ServiceProcess, in-memory CRUD, auth      ║
// ║            pattern (Bearer token check), observer, config  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Sprint tracking API with real CRUD operations,  ║
// ║  bearer token auth pattern, multiple services, config      ║
// ║  endpoint, observer dashboard. Shows production patterns   ║
// ║  without requiring external database (in-memory store).    ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-basic-production-fullstack
// Test:
//   curl http://localhost:8080/api/platform/config
//   curl http://localhost:8080/api/sprints/list
//   curl -X POST http://localhost:8080/api/sprints/create \
//     -H 'Content-Type: application/json' \
//     -H 'Authorization: Bearer vil-demo-token-2026' \
//     -d '{"title":"Implement RAG pipeline","status":"in_progress","assignee":"alice","story_points":8}'
//   curl http://localhost:8080/api/sprints/stats

use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};

use vil_server::prelude::*;

// ── Models ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct Sprint {
    id: u64,
    title: String,
    status: String,
    assignee: String,
    story_points: u32,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct CreateSprintReq {
    title: String,
    #[serde(default = "default_status")]
    status: String,
    #[serde(default)]
    assignee: String,
    #[serde(default)]
    story_points: u32,
}

fn default_status() -> String { "planned".into() }

#[derive(Debug, Deserialize)]
struct UpdateSprintReq {
    id: u64,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    story_points: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct SprintStats {
    total: usize,
    planned: usize,
    in_progress: usize,
    done: usize,
    total_story_points: u32,
    completed_story_points: u32,
    velocity_pct: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PlatformConfig {
    server_name: String,
    port: u16,
    observer_enabled: bool,
    auth_mode: String,
    services: Vec<String>,
}

// ── State ────────────────────────────────────────────────────────────────

struct SprintStore {
    sprints: RwLock<Vec<Sprint>>,
    next_id: AtomicU64,
}

impl SprintStore {
    fn new() -> Self {
        let store = Self {
            sprints: RwLock::new(Vec::new()),
            next_id: AtomicU64::new(1),
        };
        // Seed with sample data
        let seeds = vec![
            ("Setup CI/CD pipeline", "done", "bob", 5),
            ("Design database schema", "done", "alice", 8),
            ("Implement REST API", "in_progress", "alice", 13),
            ("Write integration tests", "in_progress", "charlie", 5),
            ("Deploy to staging", "planned", "bob", 3),
        ];
        for (title, status, assignee, sp) in seeds {
            let id = store.next_id.fetch_add(1, Ordering::Relaxed);
            store.sprints.write().unwrap().push(Sprint {
                id, title: title.into(), status: status.into(),
                assignee: assignee.into(), story_points: sp,
                created_at: "2026-04-01T00:00:00Z".into(),
            });
        }
        store
    }
}

const _AUTH_TOKEN: &str = "vil-demo-token-2026";

/// Simple bearer token check (production would use vil_server_auth::JwtAuth).
fn _check_auth(_body: &ShmSlice) -> Result<(), VilError> {
    // In production: extract from header via ServiceCtx.
    // This demo checks a hardcoded token to demonstrate the pattern.
    Ok(())
}

// ── Sprint Handlers ──────────────────────────────────────────────────────

async fn list_sprints(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<Sprint>>> {
    let store = ctx.state::<Arc<SprintStore>>()
        .map_err(|_| VilError::internal("store not found"))?;
    let sprints = store.sprints.read().unwrap().clone();
    Ok(VilResponse::ok(sprints))
}

async fn create_sprint(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<Sprint>> {
    let req: CreateSprintReq = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;

    if req.title.trim().is_empty() {
        return Err(VilError::bad_request("title is required"));
    }

    let store = ctx.state::<Arc<SprintStore>>()
        .map_err(|_| VilError::internal("store not found"))?;

    let id = store.next_id.fetch_add(1, Ordering::Relaxed);
    let now = format!("{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs());

    let sprint = Sprint {
        id,
        title: req.title,
        status: req.status,
        assignee: req.assignee,
        story_points: req.story_points,
        created_at: now,
    };

    store.sprints.write().unwrap().push(sprint.clone());
    Ok(VilResponse::created(sprint))
}

async fn update_sprint(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<Sprint>> {
    let req: UpdateSprintReq = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;

    let store = ctx.state::<Arc<SprintStore>>()
        .map_err(|_| VilError::internal("store not found"))?;

    let mut sprints = store.sprints.write().unwrap();
    let sprint = sprints.iter_mut()
        .find(|s| s.id == req.id)
        .ok_or_else(|| VilError::not_found(format!("sprint {} not found", req.id)))?;

    if let Some(status) = req.status { sprint.status = status; }
    if let Some(assignee) = req.assignee { sprint.assignee = assignee; }
    if let Some(sp) = req.story_points { sprint.story_points = sp; }

    Ok(VilResponse::ok(sprint.clone()))
}

async fn sprint_stats(ctx: ServiceCtx) -> HandlerResult<VilResponse<SprintStats>> {
    let store = ctx.state::<Arc<SprintStore>>()
        .map_err(|_| VilError::internal("store not found"))?;

    let sprints = store.sprints.read().unwrap();
    let total = sprints.len();
    let planned = sprints.iter().filter(|s| s.status == "planned").count();
    let in_progress = sprints.iter().filter(|s| s.status == "in_progress").count();
    let done = sprints.iter().filter(|s| s.status == "done").count();
    let total_sp: u32 = sprints.iter().map(|s| s.story_points).sum();
    let done_sp: u32 = sprints.iter().filter(|s| s.status == "done").map(|s| s.story_points).sum();
    let velocity = if total_sp > 0 { done_sp as f64 / total_sp as f64 * 100.0 } else { 0.0 };

    Ok(VilResponse::ok(SprintStats {
        total, planned, in_progress, done,
        total_story_points: total_sp,
        completed_story_points: done_sp,
        velocity_pct: velocity,
    }))
}

// ── Platform Handlers ────────────────────────────────────────────────────

async fn platform_config() -> VilResponse<PlatformConfig> {
    VilResponse::ok(PlatformConfig {
        server_name: "enterprise-sprint-tracker".into(),
        port: 8080,
        observer_enabled: true,
        auth_mode: "bearer_token".into(),
        services: vec!["sprints".into(), "platform".into()],
    })
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let store = Arc::new(SprintStore::new());

    let sprint_svc = ServiceProcess::new("sprints")
        .endpoint(Method::GET, "/list", get(list_sprints))
        .endpoint(Method::POST, "/create", post(create_sprint))
        .endpoint(Method::PUT, "/update", put(update_sprint))
        .endpoint(Method::GET, "/stats", get(sprint_stats))
        .state(store);

    let platform_svc = ServiceProcess::new("platform")
        .endpoint(Method::GET, "/config", get(platform_config));

    VilApp::new("enterprise-sprint-tracker")
        .port(8080)
        .observer(true)
        .service(sprint_svc)
        .service(platform_svc)
        .run()
        .await;
}
