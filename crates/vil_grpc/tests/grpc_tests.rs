// =============================================================================
// V9 gRPC — Unit Tests
// =============================================================================

#[test]
fn test_grpc_config_defaults() {
    use vil_grpc::GrpcServerConfig;
    let config = GrpcServerConfig::default();
    assert_eq!(config.port, 50051);
    assert_eq!(config.max_message_size, 4 * 1024 * 1024);
    assert!(config.health_check);
    assert!(config.reflection);
    assert_eq!(config.max_concurrent_streams, 200);
}

#[test]
fn test_gateway_builder() {
    use vil_grpc::GrpcGatewayBuilder;
    let gw = GrpcGatewayBuilder::new()
        .listen(9000)
        .health_check(true)
        .reflection(false)
        .max_message_size(8 * 1024 * 1024);

    assert_eq!(gw.config().port, 9000);
    assert!(gw.config().health_check);
    assert!(!gw.config().reflection);
    assert_eq!(gw.config().max_message_size, 8 * 1024 * 1024);

    let addr = gw.addr();
    assert_eq!(addr.port(), 9000);
}

#[test]
fn test_gateway_builder_default() {
    use vil_grpc::GrpcGatewayBuilder;
    let gw = GrpcGatewayBuilder::default();
    assert_eq!(gw.config().port, 50051);
}

#[test]
fn test_health_reporter() {
    use vil_grpc::health::HealthReporter;
    let hr = HealthReporter::new();
    assert!(hr.is_serving());

    hr.set_serving(false);
    assert!(!hr.is_serving());

    hr.set_serving(true);
    assert!(hr.is_serving());
}

#[test]
fn test_grpc_metrics() {
    use vil_grpc::metrics::GrpcMetrics;
    let m = GrpcMetrics::new();
    assert_eq!(m.method_count(), 0);

    m.record("/myservice.MyService/GetOrder", 150, false);
    m.record("/myservice.MyService/GetOrder", 200, false);
    m.record("/myservice.MyService/ListOrders", 500, true);

    assert_eq!(m.method_count(), 2);

    let prom = m.to_prometheus();
    assert!(prom.contains("vil_grpc_requests_total"));
    assert!(prom.contains("GetOrder"));
    assert!(prom.contains("vil_grpc_errors_total"));
}
