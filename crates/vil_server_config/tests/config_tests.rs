// =============================================================================
// Config System — Unit Tests
// =============================================================================

// ==================== Gateway Config ====================

#[test]
fn test_gateway_config_defaults() {
    use vil_server_config::GatewayConfig;
    let config = GatewayConfig::default();
    assert_eq!(config.gateway.port, 3080);
    assert_eq!(config.gateway.host, "0.0.0.0");
    assert_eq!(config.gateway.path, "/trigger");
    assert_eq!(config.gateway.upstream.format, "sse");
    assert!(config.runtime.shm.enabled);
    assert_eq!(config.logging.level, "info");
    assert!(!config.grpc.enabled);
}

#[test]
fn test_gateway_config_parse() {
    use vil_server_config::GatewayConfig;
    let yaml = r#"
gateway:
  port: 4000
  upstream:
    url: "http://localhost:8000/api"
    format: raw
    timeout_secs: 10
logging:
  level: debug
"#;
    let config = GatewayConfig::from_str(yaml).unwrap();
    assert_eq!(config.gateway.port, 4000);
    assert_eq!(config.gateway.upstream.url, "http://localhost:8000/api");
    assert_eq!(config.gateway.upstream.format, "raw");
    assert_eq!(config.gateway.upstream.timeout_secs, 10);
    assert_eq!(config.logging.level, "debug");
}

#[test]
fn test_gateway_config_empty() {
    use vil_server_config::GatewayConfig;
    let config = GatewayConfig::from_str("{}").unwrap();
    assert_eq!(config.gateway.port, 3080); // default
}

// ==================== Full Server Config ====================

#[test]
fn test_server_config_defaults() {
    use vil_server_config::FullServerConfig;
    let config = FullServerConfig::default();
    assert_eq!(config.server.name, "vil-server");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.server.request_timeout_secs, 30);
    assert!(config.shm.enabled);
    assert_eq!(config.shm.pool_size, "64MB");
    assert_eq!(config.shm.reset_threshold_pct, 80);
    assert_eq!(config.mesh.mode, "unified");
    assert_eq!(config.mesh.channels.trigger.buffer_size, 1024);
    assert_eq!(config.mesh.channels.control.buffer_size, 256);
    assert!(config.middleware.request_tracker.enabled);
    assert_eq!(config.middleware.handler_metrics.sample_rate, 1);
    assert!(!config.security.jwt.enabled);
    assert!(!config.session.enabled);
    assert_eq!(config.performance.metrics_sample_rate, 1);
    assert!(!config.grpc.enabled);
    assert!(!config.graphql.enabled);
}

#[test]
fn test_server_config_parse_full() {
    use vil_server_config::FullServerConfig;
    let yaml = r#"
server:
  name: my-platform
  port: 3000
  workers: 4
  metrics_port: 9090
shm:
  pool_size: "128MB"
  reset_threshold_pct: 90
mesh:
  mode: unified
  channels:
    trigger:
      buffer_size: 2048
      shm_region_size: "8MB"
    data:
      buffer_size: 4096
    control:
      buffer_size: 512
  routes:
    - from: auth
      to: orders
      lane: trigger
services:
  - name: auth
    visibility: public
    prefix: /auth
  - name: orders
    visibility: public
middleware:
  handler_metrics:
    sample_rate: 10
  tracing:
    sample_rate: 100
  compression:
    enabled: true
security:
  jwt:
    enabled: true
    algorithm: HS256
  rate_limit:
    enabled: true
    max_requests: 500
    window_secs: 30
session:
  enabled: true
  ttl_secs: 3600
grpc:
  enabled: true
  port: 50051
graphql:
  enabled: true
  max_depth: 5
plugins:
  active:
    - name: vil_db_sqlx
      config_file: plugins/db.yaml
"#;
    let config = FullServerConfig::from_str(yaml).unwrap();
    assert_eq!(config.server.name, "my-platform");
    assert_eq!(config.server.port, 3000);
    assert_eq!(config.server.workers, 4);
    assert_eq!(config.server.metrics_port, Some(9090));
    assert_eq!(config.shm.pool_size, "128MB");
    assert_eq!(config.shm.reset_threshold_pct, 90);
    assert_eq!(config.mesh.channels.trigger.buffer_size, 2048);
    assert_eq!(config.mesh.channels.control.buffer_size, 512);
    assert_eq!(config.mesh.routes.len(), 1);
    assert_eq!(config.services.len(), 2);
    assert_eq!(config.middleware.handler_metrics.sample_rate, 10);
    assert_eq!(config.middleware.tracing.sample_rate, 100);
    assert!(config.middleware.compression.enabled);
    assert!(config.security.jwt.enabled);
    assert_eq!(config.security.rate_limit.max_requests, 500);
    assert!(config.session.enabled);
    assert_eq!(config.session.ttl_secs, 3600);
    assert!(config.grpc.enabled);
    assert!(config.graphql.enabled);
    assert_eq!(config.graphql.max_depth, 5);
    assert_eq!(config.plugins.active.len(), 1);
    assert_eq!(config.plugins.active[0].name, "vil_db_sqlx");
}

#[test]
fn test_server_config_minimal() {
    use vil_server_config::FullServerConfig;
    let config = FullServerConfig::from_str("{}").unwrap();
    assert_eq!(config.server.port, 8080);
    assert!(config.shm.enabled);
}

#[test]
fn test_parse_size() {
    use vil_server_config::FullServerConfig;
    assert_eq!(FullServerConfig::parse_size("64MB"), 64 * 1024 * 1024);
    assert_eq!(FullServerConfig::parse_size("1GB"), 1024 * 1024 * 1024);
    assert_eq!(FullServerConfig::parse_size("512KB"), 512 * 1024);
    assert_eq!(FullServerConfig::parse_size("1024"), 1024);
}

#[test]
fn test_server_config_env_override() {
    use vil_server_config::FullServerConfig;
    std::env::set_var("VIL_SERVER_PORT", "9999");
    std::env::set_var("VIL_LOG_LEVEL", "debug");

    let mut config = FullServerConfig::default();
    config.apply_env_overrides();

    assert_eq!(config.server.port, 9999);
    assert_eq!(config.logging.level, "debug");

    std::env::remove_var("VIL_SERVER_PORT");
    std::env::remove_var("VIL_LOG_LEVEL");
}

// ==================== Channel Config ====================

#[test]
fn test_channel_defaults_differentiated() {
    use vil_server_config::FullServerConfig;
    let config = FullServerConfig::default();

    // Trigger and Data have same buffer, Control is smaller
    assert_eq!(config.mesh.channels.trigger.buffer_size, 1024);
    assert_eq!(config.mesh.channels.data.buffer_size, 1024);
    assert_eq!(config.mesh.channels.control.buffer_size, 256);

    // Data has larger SHM region than Trigger
    assert_eq!(config.mesh.channels.data.shm_region_size, "16MB");
    assert_eq!(config.mesh.channels.trigger.shm_region_size, "4MB");
    assert_eq!(config.mesh.channels.control.shm_region_size, "1MB");
}

// ==================== Security Config ====================

#[test]
fn test_security_all_disabled_by_default() {
    use vil_server_config::FullServerConfig;
    let config = FullServerConfig::default();
    assert!(!config.security.jwt.enabled);
    assert!(!config.security.rate_limit.enabled);
    assert!(!config.security.csrf.enabled);
    assert!(!config.security.brute_force.enabled);
}

// ==================== Plugin Config Reference ====================

#[test]
fn test_plugin_ref_parse() {
    use vil_server_config::FullServerConfig;
    let yaml = r#"
plugins:
  directory: "/custom/plugins"
  active:
    - name: vil_db_sqlx
      config_file: plugins/db.yaml
    - name: vil_mq_kafka
      config_file: plugins/kafka.yaml
"#;
    let config = FullServerConfig::from_str(yaml).unwrap();
    assert_eq!(config.plugins.directory, "/custom/plugins");
    assert_eq!(config.plugins.active.len(), 2);
    assert_eq!(config.plugins.active[0].config_file, Some("plugins/db.yaml".into()));
    assert_eq!(config.plugins.active[1].name, "vil_mq_kafka");
}
