// =============================================================================
// V8 Kubernetes Operator — Unit Tests
// =============================================================================

#[test]
fn test_crd_deserialization() {
    use vil_operator::crd::*;

    let yaml = r#"
apiVersion: vil.dev/v1alpha1
kind: VilServer
metadata:
  name: test-server
spec:
  image: "ghcr.io/oceanos-id/vil-server:0.1.0"
  replicas: 3
  port: 8080
  metricsPort: 9090
  shm:
    enabled: true
    sizeLimit: "512Mi"
  services:
    - name: auth
      visibility: public
    - name: orders
      visibility: public
      prefix: /api
  mesh:
    mode: unified
    routes:
      - from: auth
        to: orders
        lane: trigger
"#;

    let server: VilServer = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(server.spec.replicas, 3);
    assert_eq!(server.spec.port, 8080);
    assert_eq!(server.spec.metrics_port, 9090);
    assert!(server.spec.shm.enabled);
    assert_eq!(server.spec.shm.size_limit, "512Mi");
    assert_eq!(server.spec.services.len(), 2);
    assert_eq!(server.spec.services[0].name, "auth");
    assert_eq!(server.spec.mesh.mode, "unified");
    assert_eq!(server.spec.mesh.routes.len(), 1);
    assert_eq!(server.spec.mesh.routes[0].from, "auth");
    assert_eq!(server.spec.mesh.routes[0].to, "orders");
}

#[test]
fn test_crd_defaults() {
    use vil_operator::crd::*;

    // Test VilServerSpec serde defaults
    let spec: VilServerSpec = serde_yaml::from_str("{}").unwrap();
    assert_eq!(spec.replicas, 1);
    assert_eq!(spec.port, 8080);
    assert_eq!(spec.metrics_port, 9090);
    assert!(spec.services.is_empty());

    // Test ShmSpec defaults
    let shm = ShmSpec::default();
    assert!(shm.enabled);
    assert_eq!(shm.size_limit, "256Mi");
}

#[test]
fn test_resource_generation_deployment() {
    use vil_operator::crd::VilServerSpec;
    use vil_operator::resources::generate_deployment;

    let spec = VilServerSpec {
        image: "test:latest".into(),
        replicas: 2,
        port: 3000,
        metrics_port: 9090,
        shm: vil_operator::crd::ShmSpec { enabled: true, size_limit: "128Mi".into() },
        services: Vec::new(),
        mesh: Default::default(),
        resources: None,
        autoscaling: None,
    };

    let deployment = generate_deployment("my-app", "default", &spec);
    assert_eq!(deployment["kind"], "Deployment");
    assert_eq!(deployment["metadata"]["name"], "my-app");
    assert_eq!(deployment["metadata"]["namespace"], "default");
    assert_eq!(deployment["spec"]["replicas"], 2);

    // Check SHM volume exists
    let volumes = deployment["spec"]["template"]["spec"]["volumes"].as_array().unwrap();
    assert!(volumes.iter().any(|v| v["name"] == "shm"));

    // Check container port
    let container = &deployment["spec"]["template"]["spec"]["containers"][0];
    assert_eq!(container["image"], "test:latest");
    assert!(container["ports"].as_array().unwrap().iter().any(|p| p["containerPort"] == 3000));
}

#[test]
fn test_resource_generation_service() {
    use vil_operator::crd::VilServerSpec;
    use vil_operator::resources::generate_service;

    let spec = VilServerSpec {
        image: "test:latest".into(),
        replicas: 1,
        port: 8080,
        metrics_port: 9090,
        shm: Default::default(),
        services: Vec::new(),
        mesh: Default::default(),
        resources: None,
        autoscaling: None,
    };

    let service = generate_service("my-app", "default", &spec);
    assert_eq!(service["kind"], "Service");
    assert_eq!(service["spec"]["type"], "ClusterIP");
    let ports = service["spec"]["ports"].as_array().unwrap();
    assert_eq!(ports.len(), 2); // http + metrics
}

#[test]
fn test_deployment_no_shm() {
    use vil_operator::crd::*;
    use vil_operator::resources::generate_deployment;

    let spec = VilServerSpec {
        image: "test:latest".into(),
        replicas: 1,
        port: 8080,
        metrics_port: 9090,
        shm: ShmSpec { enabled: false, size_limit: "256Mi".into() },
        services: Vec::new(),
        mesh: Default::default(),
        resources: None,
        autoscaling: None,
    };

    let deployment = generate_deployment("my-app", "default", &spec);
    let volumes = deployment["spec"]["template"]["spec"]["volumes"].as_array().unwrap();
    assert!(volumes.is_empty()); // No SHM volume when disabled
}

#[test]
fn test_status_helpers() {
    use vil_operator::status::*;

    let running = status_running(3);
    assert_eq!(running.phase, "Running");
    assert_eq!(running.replicas, 3);
    assert_eq!(running.ready_replicas, 3);

    let pending = status_pending("Waiting for pods");
    assert_eq!(pending.phase, "Pending");
    assert!(pending.message.contains("Waiting"));

    let error = status_error("Image pull failed");
    assert_eq!(error.phase, "Error");
}

#[test]
fn test_controller_action_create() {
    use vil_operator::crd::*;
    use vil_operator::controller::*;

    let yaml = r#"
apiVersion: vil.dev/v1alpha1
kind: VilServer
metadata:
  name: new-server
spec:
  replicas: 1
"#;
    let server: VilServer = serde_yaml::from_str(yaml).unwrap();
    let action = determine_action(&server);
    assert!(matches!(action, ReconcileAction::Create));
}
