// =============================================================================
// VIL Server — End-to-End Tests
// =============================================================================
//
// Tests full server lifecycle without network (via tower::ServiceExt::oneshot).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use tower::ServiceExt;
use vil_server_core::AppState;

fn build_full_app() -> Router {
    let state = AppState::new("e2e-test");

    Router::new()
        .route("/", get(|| async { "E2E OK" }))
        .route(
            "/json",
            get(|| async { axum::Json(serde_json::json!({"e2e": true, "version": "4.0.0"})) }),
        )
        .route("/echo", post(|body: String| async move { body }))
        .route(
            "/health",
            get(|| async { axum::Json(serde_json::json!({"status": "healthy"})) }),
        )
        .with_state(state)
}

// ==================== E2E: Server Lifecycle ====================

#[tokio::test]
async fn e2e_server_hello() {
    let app = build_full_app();
    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"E2E OK");
}

#[tokio::test]
async fn e2e_server_json() {
    let app = build_full_app();
    let resp = app
        .oneshot(Request::builder().uri("/json").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["e2e"], true);
    assert_eq!(json["version"], "4.0.0");
}

#[tokio::test]
async fn e2e_server_echo() {
    let app = build_full_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/echo")
                .header("content-type", "text/plain")
                .body(Body::from("hello e2e"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"hello e2e");
}

#[tokio::test]
async fn e2e_server_health() {
    let app = build_full_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "healthy");
}

#[tokio::test]
async fn e2e_server_not_found() {
    let app = build_full_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ==================== E2E: SHM Zero-Copy ====================

#[tokio::test]
async fn e2e_shm_context() {
    let state = AppState::new("shm-test");

    // Verify SHM pool is initialized
    let pool_stats = state.shm_pool().stats();
    assert!(pool_stats.capacity > 0);
    assert_eq!(pool_stats.total_allocs, 0);
}

#[tokio::test]
async fn e2e_shm_pool_alloc() {
    let state = AppState::new("shm-alloc-test");
    let pool = state.shm_pool();

    // Allocate and write
    let data = b"hello shm e2e test";
    let result = pool.alloc_and_write(data);
    assert!(result.is_some());

    let stats = pool.stats();
    assert_eq!(stats.total_allocs, 1);
    assert!(stats.used > 0);
}

// ==================== E2E: Plugin Manager ====================

#[tokio::test]
async fn e2e_plugin_manager_lifecycle() {
    let state = AppState::new("plugin-test");
    let mgr = state.plugin_manager();

    // Initially empty
    assert_eq!(mgr.plugin_count(), 0);
    assert!(mgr.list_plugins().is_empty());

    // Install a plugin
    use vil_server_core::plugin_manifest::*;
    let manifest = PluginManifest {
        name: "test_plugin".into(),
        version: "0.1.0".into(),
        description: "Test plugin".into(),
        plugin_type: PluginType::Custom,
        tier: PluginTier::Community,
        author: "test".into(),
        license: "Apache-2.0".into(),
        homepage: String::new(),
        signature: None,
        config_schema: std::collections::HashMap::new(),
        health_check: HealthCheckConfig::default(),
        metrics: Vec::new(),
        admin_ui: AdminUiHints::default(),
        provides: Vec::new(),
        requires: Vec::new(),
    };

    let result = mgr.install(manifest);
    assert!(result.is_ok());
    assert_eq!(mgr.plugin_count(), 1);

    // Enable
    assert!(mgr.enable("test_plugin").is_ok());
    assert!(mgr.is_enabled("test_plugin"));

    // Disable
    assert!(mgr.disable("test_plugin").is_ok());
    assert!(!mgr.is_enabled("test_plugin"));

    // Remove
    assert!(mgr.remove("test_plugin").is_ok());
    assert_eq!(mgr.plugin_count(), 0);
}

// ==================== E2E: Custom Metrics ====================

#[tokio::test]
async fn e2e_custom_metrics() {
    let state = AppState::new("metrics-test");
    let metrics = state.custom_metrics();

    metrics.register_counter("e2e_requests", "E2E request counter");
    metrics.register_gauge("e2e_connections", "E2E active connections");

    metrics.inc("e2e_requests");
    metrics.inc("e2e_requests");
    metrics.gauge_set("e2e_connections", 5);

    assert_eq!(metrics.counter_value("e2e_requests"), 2);
    assert_eq!(metrics.gauge_value("e2e_connections"), 5);

    let prom = metrics.to_prometheus();
    assert!(prom.contains("e2e_requests"));
    assert!(prom.contains("e2e_connections"));
}

// ==================== E2E: Error Tracker ====================

#[tokio::test]
async fn e2e_error_tracker() {
    let state = AppState::new("error-test");
    let tracker = state.error_tracker();

    tracker.record("GET", "/api/fail", 500, "Internal error", Some("req-1"));
    tracker.record("GET", "/api/fail", 500, "Internal error", Some("req-2"));
    tracker.record("POST", "/api/other", 400, "Bad request", None);

    assert_eq!(tracker.error_count(), 3);
    assert_eq!(tracker.pattern_count(), 2);

    let top = tracker.top_patterns(10);
    assert_eq!(top[0].count, 2); // /api/fail hit 2x
}

// ==================== E2E: Span Collector ====================

#[tokio::test]
async fn e2e_span_collector() {
    let state = AppState::new("trace-test");
    let collector = state.span_collector();

    use vil_server_core::otel::*;
    let span = SpanBuilder::new("e2e_op", SpanKind::Server, "e2e-service")
        .attr("test", "true")
        .finish(SpanStatus::Ok);

    collector.record(span);
    assert_eq!(collector.buffered(), 1);
    assert_eq!(collector.total_collected(), 1);

    let recent = collector.recent(10);
    assert_eq!(recent[0].name, "e2e_op");
    assert_eq!(recent[0].service_name, "e2e-service");
}

// ==================== E2E: DB Semantic Primitives ====================

#[test]
fn e2e_semantic_primitives_zero_cost() {
    use vil_db_semantic::*;

    // All stack-allocated, total < 1 cache line
    let ds = DatasourceRef::new("main_db");
    let tx = TxScope::ReadOnly;
    let cap = DbCapability::SQL_STANDARD;
    let tier = PortabilityTier::P0;
    let cache = CachePolicy::Ttl(60);

    assert_eq!(ds.name(), "main_db");
    assert_eq!(tx, TxScope::ReadOnly);
    assert!(cap.contains(DbCapability::BASIC_CRUD));
    assert_eq!(tier, PortabilityTier::P0);
    assert!(matches!(cache, CachePolicy::Ttl(60)));

    // Prove zero-cost: total < 64 bytes (1 cache line)
    let total = std::mem::size_of_val(&ds)
        + std::mem::size_of_val(&tx)
        + std::mem::size_of_val(&cap)
        + std::mem::size_of_val(&tier)
        + std::mem::size_of_val(&cache);
    assert!(total <= 64);
}

// ==================== E2E: Multiple Concurrent Requests ====================

#[tokio::test]
async fn e2e_concurrent_requests() {
    let app = build_full_app();

    // Fire 100 concurrent requests
    let mut handles = Vec::new();
    for i in 0..100 {
        let app = app.clone();
        handles.push(tokio::spawn(async move {
            let resp = app
                .oneshot(Request::builder().uri("/json").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            i
        }));
    }

    let mut completed = 0;
    for h in handles {
        h.await.unwrap();
        completed += 1;
    }
    assert_eq!(completed, 100);
}
