// =============================================================================
// VIL Server — Integration Tests
// =============================================================================
//
// Tests the full vil-server stack without network overhead.
// Uses TestClient for direct router dispatch.

use axum::routing::get;
use vil_server_core::{AppState, Router, VilServer};
use vil_server_test::TestClient;

fn build_test_app() -> Router {
    let state = AppState::new("test-server");

    Router::new()
        .route("/", get(|| async { "Hello from test!" }))
        .route("/json", get(|| async {
            axum::Json(serde_json::json!({
                "status": "ok",
                "service": "test"
            }))
        }))
        .route("/echo", axum::routing::post(|body: String| async move { body }))
        .with_state(state)
}

#[tokio::test]
async fn test_hello() {
    let app = build_test_app();
    let client = TestClient::new(app);

    let resp = client.get("/").await;
    resp.assert_ok();
    assert_eq!(resp.text(), "Hello from test!");
}

#[tokio::test]
async fn test_json_response() {
    let app = build_test_app();
    let client = TestClient::new(app);

    let resp = client.get("/json").await;
    resp.assert_ok();

    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "test");
}

#[tokio::test]
async fn test_echo() {
    let app = build_test_app();
    let client = TestClient::new(app);

    let resp = client.post_json("/echo", "hello world").await;
    resp.assert_ok();
    assert_eq!(resp.text(), "hello world");
}

#[tokio::test]
async fn test_not_found() {
    let app = build_test_app();
    let client = TestClient::new(app);

    let resp = client.get("/nonexistent").await;
    resp.assert_not_found();
}

// =============================================================================
// Unit tests for individual components
// =============================================================================

#[test]
fn test_circuit_breaker_states() {
    use vil_server_auth::circuit_breaker::*;

    let cb = CircuitBreaker::new("test-service", CircuitBreakerConfig {
        failure_threshold: 3,
        ..Default::default()
    });

    // Initially closed
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.check().is_ok());

    // Record failures
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Closed); // Still below threshold

    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open); // Threshold reached
    assert!(cb.check().is_err()); // Requests rejected

    // Reset
    cb.reset();
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.check().is_ok());
}

#[test]
fn test_rate_limiter() {
    use vil_server_auth::RateLimit;
    use std::net::{IpAddr, Ipv4Addr};

    let limiter = RateLimit::new(5, std::time::Duration::from_secs(60));
    let test_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
    let other_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

    // First 5 requests should pass
    for _ in 0..5 {
        assert!(limiter.check(test_ip));
    }

    // 6th request should be rate limited
    assert!(!limiter.check(test_ip));

    // Different IP should work
    assert!(limiter.check(other_ip));
}

#[test]
fn test_backpressure_controller() {
    use vil_server_mesh::backpressure::*;

    let controller = BackpressureController::new("test", 10);

    // Should be accepting initially
    assert!(controller.is_accepting());
    assert_eq!(controller.in_flight(), 0);

    // Fill up to high watermark (8 = 80% of 10)
    for _ in 0..7 {
        let _ = controller.request_enter();
    }
    assert!(controller.is_accepting());

    // Cross high watermark → throttle
    let signal = controller.request_enter();
    assert!(signal.is_some());

    // Hit max → pause
    let _ = controller.request_enter();
    let signal = controller.request_enter();
    assert!(matches!(signal, Some(BackpressureSignal::Pause)));
    assert!(!controller.is_accepting());

    // Drain back down — need to go below low_watermark (5) while paused
    // Currently at 10, need to reach <= 5
    // Each request_exit decrements by 1
    for _ in 0..4 {
        let _ = controller.request_exit();  // 10→9→8→7→6
    }
    assert!(!controller.is_accepting()); // Still above low watermark

    // Cross below low watermark → resume
    let signal = controller.request_exit(); // 6→5 = low_watermark
    assert!(matches!(signal, Some(BackpressureSignal::Resume)));
    assert!(controller.is_accepting());
}

#[test]
fn test_yaml_config_parse() {
    use vil_server_mesh::yaml_config::*;

    let yaml = r#"
server:
  name: test-app
  port: 3000
services:
  - name: api
    visibility: public
    prefix: /api
  - name: worker
    visibility: internal
mesh:
  mode: unified
  routes:
    - from: api
      to: worker
      lane: trigger
"#;

    let config = VilServerYaml::from_str(yaml).unwrap();
    assert_eq!(config.server.name, "test-app");
    assert_eq!(config.server.port, 3000);
    assert_eq!(config.services.len(), 2);
    assert_eq!(config.mesh.routes.len(), 1);
    assert!(config.validate().is_ok());
}

#[test]
fn test_yaml_config_validation_error() {
    use vil_server_mesh::yaml_config::*;

    let yaml = r#"
server:
  name: test
services:
  - name: api
mesh:
  routes:
    - from: api
      to: nonexistent
      lane: trigger
"#;

    let config = VilServerYaml::from_str(yaml).unwrap();
    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err()[0].contains("nonexistent"));
}

#[test]
fn test_openapi_builder() {
    use vil_server_web::openapi::OpenApiBuilder;

    let spec = OpenApiBuilder::new("Test API", "1.0.0")
        .description("A test API")
        .server("http://localhost:8080", None)
        .get("/health", "Health check", "healthCheck")
        .post("/users", "Create user", "createUser")
        .build_json();

    let parsed: serde_json::Value = serde_json::from_str(&spec).unwrap();
    assert_eq!(parsed["openapi"], "3.0.3");
    assert_eq!(parsed["info"]["title"], "Test API");
    assert!(parsed["paths"]["/health"]["get"].is_object());
    assert!(parsed["paths"]["/users"]["post"].is_object());
}
