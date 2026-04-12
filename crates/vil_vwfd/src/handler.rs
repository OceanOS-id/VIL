//! VIL VWFD Pipeline — shared webhook pipeline with WorkflowRouter.
//!
//! Follows vflow pattern: single HttpSink (wildcard) → VwfdKernel → HttpSink.
//! GenericToken transport, async executor, lock-free router.

use crate::graph::VilwGraph;
use crate::executor::{self, ExecConfig};
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};

/// Workflow metadata (used by macro-generated code).
pub struct WorkflowMeta {
    pub id: &'static str,
    pub route: &'static str,
    pub trigger_type: &'static str,
    pub node_count: usize,
}

// ── WorkflowRouter (Lock-Free Read) ─────────────────────────────────────

struct Route {
    method: String,  // GET, POST, PUT, DELETE, ANY
    path: String,
    graph: Arc<VilwGraph>,
}

/// Lock-free workflow router — hot-path reads without contention.
///
/// Read path: atomic load → Arc<Vec> → linear scan.
/// Write path (rare — provision): clone Vec, push, atomic swap.
#[derive(Clone)]
pub struct WorkflowRouter {
    routes: Arc<AtomicPtr<Arc<Vec<Route>>>>,
}

// Safety: AtomicPtr operations are the only access path, properly ordered.
unsafe impl Send for WorkflowRouter {}
unsafe impl Sync for WorkflowRouter {}

impl WorkflowRouter {
    pub fn new() -> Self {
        let empty: Arc<Vec<Route>> = Arc::new(Vec::new());
        let ptr = Box::into_raw(Box::new(empty));
        Self {
            routes: Arc::new(AtomicPtr::new(ptr)),
        }
    }

    /// Register a workflow for a method + path. Lock-free write via CAS.
    pub fn register(&self, method: String, path: String, graph: Arc<VilwGraph>) {
        let node_count = graph.node_count();
        loop {
            let old_ptr = self.routes.load(Ordering::Acquire);
            let old_arc = unsafe { &*old_ptr };

            let mut new_routes: Vec<Route> = old_arc
                .iter()
                .filter(|r| !(r.method == method && r.path == path))
                .map(|r| Route { method: r.method.clone(), path: r.path.clone(), graph: r.graph.clone() })
                .collect();
            new_routes.push(Route { method: method.clone(), path: path.clone(), graph: graph.clone() });

            let new_arc = Arc::new(new_routes);
            let new_ptr = Box::into_raw(Box::new(new_arc));

            if self.routes
                .compare_exchange(old_ptr, new_ptr, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                let _ = unsafe { Box::from_raw(old_ptr) };
                break;
            } else {
                let _ = unsafe { Box::from_raw(new_ptr) };
            }
        }
        tracing::info!("WorkflowRouter: registered {} {} ({} nodes)", method, path, node_count);
    }

    /// Lookup workflow graph by webhook path. Lock-free hot path.
    #[inline]
    /// Lookup workflow by method + path. Priority:
    /// 1. Exact method + exact path
    /// 2. Exact method + prefix path (e.g. GET /tasks/* matches GET /tasks)
    /// 3. ANY method + exact path (fallback for single-workflow routes)
    /// 4. ANY method + prefix path
    pub fn lookup(&self, method: &str, path: &str) -> Option<Arc<VilwGraph>> {
        let ptr = self.routes.load(Ordering::Acquire);
        let routes = unsafe { &*ptr };

        // 1. Exact method + exact path
        for r in routes.iter() {
            if r.method == method && r.path == path {
                return Some(r.graph.clone());
            }
        }
        // 2. Exact method + longest prefix match
        let mut best: Option<&Route> = None;
        for r in routes.iter() {
            if r.method == method && path.starts_with(&r.path) {
                if best.is_none() || r.path.len() > best.unwrap().path.len() {
                    best = Some(r);
                }
            }
        }
        if let Some(r) = best { return Some(r.graph.clone()); }
        // 3. ANY/POST (legacy) + exact path
        for r in routes.iter() {
            if (r.method == "ANY" || r.method == "POST") && r.path == path {
                return Some(r.graph.clone());
            }
        }
        // 4. ANY/POST + prefix
        for r in routes.iter() {
            if (r.method == "ANY" || r.method == "POST") && path.starts_with(&r.path) {
                return Some(r.graph.clone());
            }
        }
        None
    }

    pub fn routes(&self) -> Vec<String> {
        let ptr = self.routes.load(Ordering::Acquire);
        let routes = unsafe { &*ptr };
        routes.iter().map(|r| format!("{} {}", r.method, r.path)).collect()
    }

    pub fn count(&self) -> usize {
        let ptr = self.routes.load(Ordering::Acquire);
        let routes = unsafe { &*ptr };
        routes.len()
    }
}

impl Default for WorkflowRouter {
    fn default() -> Self { Self::new() }
}

// ── Request handling ────────────────────────────────────────────────────

/// Execute workflow for given path + input (async).
pub async fn handle_request(
    router: &WorkflowRouter,
    method: &str,
    path: &str,
    input: serde_json::Value,
    config: &ExecConfig,
) -> Result<serde_json::Value, String> {
    let graph = router.lookup(method, path)
        .ok_or_else(|| format!("no workflow for {} {}", method, path))?;

    executor::execute(&graph, input, config)
        .await
        .map(|r| r.output)
        .map_err(|e| e.to_string())
}

/// Load workflows from directory, start VilApp server, block.
pub async fn serve(workflow_dir: &str, port: u16) -> Result<(), String> {
    use vil_server_core::{
        axum::{self, extract::Extension, body::Bytes, response::IntoResponse, http::Method},
        axum::routing::post,
        Json, StatusCode, ServiceProcess, VilApp,
    };

    let load_result = crate::loader::load_dir(workflow_dir);
    if load_result.graphs.is_empty() && !load_result.errors.is_empty() {
        return Err(format!("no workflows loaded: {:?}",
            load_result.errors.iter().map(|e| &e.error).collect::<Vec<_>>()));
    }

    let router = WorkflowRouter::new();
    for g in load_result.graphs {
        let path = g.webhook_route.clone().unwrap_or_else(|| format!("/{}", g.id));
        let method = g.webhook_method.clone();
        router.register(method, path, Arc::new(g));
    }

    let routes = router.routes();
    eprintln!("vil_vwfd server on :{}", port);
    for r in &routes {
        eprintln!("  {}", r);
    }

    // Init connector pools from env vars → wire to executor
    let mut pools = crate::registry::ConnectorPools::new();
    let pool_errors = pools.init_from_env().await;
    for e in &pool_errors {
        eprintln!("  pool init warning: {}", e);
    }
    let pools = Arc::new(pools);

    let config = ExecConfig {
        connector_fn: Some(crate::registry::registry_connector_fn(pools)),
        ..Default::default()
    };

    // VilApp wildcard handler — all POSTs dispatched via WorkflowRouter
    let router_ext = Arc::new(router);
    let config_ext = Arc::new(config);

    async fn vwfd_handler(
        Extension(router): Extension<Arc<WorkflowRouter>>,
        Extension(config): Extension<Arc<ExecConfig>>,
        axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
        body: Bytes,
    ) -> impl IntoResponse {
        let path = uri.path().to_string();
        let input: serde_json::Value = serde_json::from_slice(&body)
            .unwrap_or(serde_json::json!({}));
        match handle_request(&router, "POST", &path, input, &config).await {
            Ok(output) => (StatusCode::OK, Json(output)).into_response(),
            Err(e) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e}))).into_response(),
        }
    }

    let svc = ServiceProcess::new("vwfd")
        .prefix("")
        .endpoint(Method::POST, "/*path", post(vwfd_handler))
        .extension(router_ext)
        .extension(config_ext);

    VilApp::new("vil-vwfd")
        .port(port)
        .service(svc)
        .run()
        .await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler;
    use serde_json::json;

    const WF: &str = r#"
version: "3.0"
metadata:
  id: handler-test
spec:
  activities:
    - id: trigger
      activity_type: Trigger
      trigger_config:
        trigger_type: webhook
        webhook_config: { path: /api/test }
        response_mode: buffered
        end_activity: respond
      output_variable: trigger_payload
    - id: respond
      activity_type: EndTrigger
      end_trigger_config:
        trigger_ref: trigger
        final_response:
          language: vil-expr
          source: '{"echo": trigger_payload.name, "status": "ok"}'
    - id: end
      activity_type: End
  flows:
    - { id: f1, from: { node: trigger }, to: { node: respond } }
    - { id: f2, from: { node: respond }, to: { node: end } }
"#;

    #[test]
    fn test_router_register_and_lookup() {
        let graph = compiler::compile(WF).unwrap();
        let router = WorkflowRouter::new();
        let path = graph.webhook_route.clone().unwrap_or("/test".into());
        router.register("POST".into(), path, Arc::new(graph));

        assert_eq!(router.count(), 1);
        assert!(router.lookup("POST", "/api/test").is_some());
        assert!(router.lookup("POST", "/nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_handle_request() {
        let graph = compiler::compile(WF).unwrap();
        let router = WorkflowRouter::new();
        let path = graph.webhook_route.clone().unwrap_or("/test".into());
        router.register("POST".into(), path, Arc::new(graph));

        let config = ExecConfig::default();
        let result = handle_request(&router, "POST", "/api/test", json!({"name": "Alice"}), &config).await.unwrap();
        assert_eq!(result["echo"], "Alice");
        assert_eq!(result["status"], "ok");
    }

    #[tokio::test]
    async fn test_handle_request_not_found() {
        let router = WorkflowRouter::new();
        let config = ExecConfig::default();
        let result = handle_request(&router, "POST", "/missing", json!({}), &config).await;
        assert!(result.is_err());
    }
}
