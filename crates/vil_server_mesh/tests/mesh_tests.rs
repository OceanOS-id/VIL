// =============================================================================
// VIL Server Mesh — Unit Tests
// =============================================================================

// ==================== YAML Config Tests ====================

#[cfg(test)]
mod yaml_config_tests {
    use vil_server_mesh::yaml_config::*;

    #[test]
    fn test_parse_full_config() {
        let yaml = r#"
server:
  name: test-app
  port: 3000
  metrics_port: 9090
  workers: 4
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
        assert_eq!(config.server.metrics_port, Some(9090));
        assert_eq!(config.services.len(), 2);
        assert_eq!(config.mesh.routes.len(), 1);
        assert_eq!(config.mesh.mode, DeploymentMode::Unified);
    }

    #[test]
    fn test_validation_success() {
        let yaml = r#"
services:
  - name: a
  - name: b
mesh:
  routes:
    - from: a
      to: b
"#;
        let config = VilServerYaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_unknown_service() {
        let yaml = r#"
services:
  - name: a
mesh:
  routes:
    - from: a
      to: nonexistent
"#;
        let config = VilServerYaml::from_str(yaml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("nonexistent"));
    }

    #[test]
    fn test_validation_duplicate_service() {
        let yaml = r#"
services:
  - name: api
  - name: api
"#;
        let config = VilServerYaml::from_str(yaml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("Duplicate"));
    }

    #[test]
    fn test_to_mesh_config() {
        let yaml = r#"
services:
  - name: a
  - name: b
mesh:
  routes:
    - from: a
      to: b
      lane: data
"#;
        let config = VilServerYaml::from_str(yaml).unwrap();
        let mesh = config.to_mesh_config();
        assert_eq!(mesh.routes.len(), 1);
        assert_eq!(mesh.routes[0].lane, vil_server_mesh::Lane::Data);
    }

    #[test]
    fn test_default_server_config() {
        let yaml = "{}";
        let config = VilServerYaml::from_str(yaml).unwrap();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.host, "0.0.0.0");
    }

    #[test]
    fn test_standalone_mode() {
        let yaml = r#"
mesh:
  mode: standalone
"#;
        let config = VilServerYaml::from_str(yaml).unwrap();
        assert_eq!(config.mesh.mode, DeploymentMode::Standalone);
    }
}

// ==================== Pipeline DAG Tests ====================

#[cfg(test)]
mod dag_tests {
    use vil_server_mesh::pipeline_dag::*;

    #[test]
    fn test_linear_dag() {
        let mut dag = PipelineDag::new("linear");
        dag.add_node(DagNode {
            id: "a".into(),
            handler: "h1".into(),
            depends_on: vec![],
            config: None,
        });
        dag.add_node(DagNode {
            id: "b".into(),
            handler: "h2".into(),
            depends_on: vec!["a".into()],
            config: None,
        });
        dag.add_node(DagNode {
            id: "c".into(),
            handler: "h3".into(),
            depends_on: vec!["b".into()],
            config: None,
        });

        assert!(dag.validate().is_ok());
        let plan = dag.plan().unwrap();
        assert_eq!(plan.stages.len(), 3);
        assert_eq!(plan.stages[0], vec!["a"]);
        assert_eq!(plan.stages[1], vec!["b"]);
        assert_eq!(plan.stages[2], vec!["c"]);
    }

    #[test]
    fn test_parallel_dag() {
        let mut dag = PipelineDag::new("parallel");
        dag.add_node(DagNode {
            id: "root".into(),
            handler: "h".into(),
            depends_on: vec![],
            config: None,
        });
        dag.add_node(DagNode {
            id: "a".into(),
            handler: "h".into(),
            depends_on: vec!["root".into()],
            config: None,
        });
        dag.add_node(DagNode {
            id: "b".into(),
            handler: "h".into(),
            depends_on: vec!["root".into()],
            config: None,
        });
        dag.add_node(DagNode {
            id: "c".into(),
            handler: "h".into(),
            depends_on: vec!["root".into()],
            config: None,
        });
        dag.add_node(DagNode {
            id: "merge".into(),
            handler: "h".into(),
            depends_on: vec!["a".into(), "b".into(), "c".into()],
            config: None,
        });

        let plan = dag.plan().unwrap();
        assert_eq!(plan.stages.len(), 3);
        assert_eq!(plan.stages[0], vec!["root"]);
        assert_eq!(plan.stages[1].len(), 3); // a, b, c in parallel
        assert_eq!(plan.stages[2], vec!["merge"]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut dag = PipelineDag::new("cycle");
        dag.add_node(DagNode {
            id: "a".into(),
            handler: "h".into(),
            depends_on: vec!["b".into()],
            config: None,
        });
        dag.add_node(DagNode {
            id: "b".into(),
            handler: "h".into(),
            depends_on: vec!["a".into()],
            config: None,
        });

        let result = dag.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("cycle")));
    }

    #[test]
    fn test_missing_dependency() {
        let mut dag = PipelineDag::new("missing");
        dag.add_node(DagNode {
            id: "a".into(),
            handler: "h".into(),
            depends_on: vec!["ghost".into()],
            config: None,
        });

        let result = dag.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("ghost"));
    }

    #[test]
    fn test_entry_exit_nodes() {
        let mut dag = PipelineDag::new("test");
        dag.add_node(DagNode {
            id: "entry1".into(),
            handler: "h".into(),
            depends_on: vec![],
            config: None,
        });
        dag.add_node(DagNode {
            id: "entry2".into(),
            handler: "h".into(),
            depends_on: vec![],
            config: None,
        });
        dag.add_node(DagNode {
            id: "middle".into(),
            handler: "h".into(),
            depends_on: vec!["entry1".into()],
            config: None,
        });
        dag.add_node(DagNode {
            id: "exit".into(),
            handler: "h".into(),
            depends_on: vec!["middle".into(), "entry2".into()],
            config: None,
        });

        let entries = dag.entry_nodes();
        assert_eq!(entries.len(), 2);

        let exits = dag.exit_nodes();
        assert_eq!(exits.len(), 1);
        assert_eq!(exits[0].id, "exit");
    }

    #[test]
    fn test_duplicate_node_id() {
        let mut dag = PipelineDag::new("dup");
        dag.add_node(DagNode {
            id: "a".into(),
            handler: "h".into(),
            depends_on: vec![],
            config: None,
        });
        dag.add_node(DagNode {
            id: "a".into(),
            handler: "h".into(),
            depends_on: vec![],
            config: None,
        });

        let result = dag.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("Duplicate")));
    }
}

// ==================== DLQ Tests ====================

#[cfg(test)]
mod dlq_tests {
    use vil_server_mesh::dlq::DeadLetterQueue;
    use vil_server_mesh::Lane;

    #[test]
    fn test_enqueue() {
        let dlq = DeadLetterQueue::new(100);
        dlq.enqueue("svc_a", "svc_b", Lane::Data, 1024, "timeout", 0);
        assert_eq!(dlq.depth(), 1);
        assert_eq!(dlq.total_enqueued(), 1);
    }

    #[test]
    fn test_recent() {
        let dlq = DeadLetterQueue::new(100);
        for i in 0..5 {
            dlq.enqueue("a", "b", Lane::Trigger, i * 100, &format!("err_{}", i), 0);
        }
        let recent = dlq.recent(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_get_by_id() {
        let dlq = DeadLetterQueue::new(100);
        dlq.enqueue("a", "b", Lane::Data, 512, "error", 0);
        let letter = dlq.get(1);
        assert!(letter.is_some());
        assert_eq!(letter.unwrap().from_service, "a");
    }

    #[test]
    fn test_ring_buffer() {
        let dlq = DeadLetterQueue::new(3);
        for i in 0..5 {
            dlq.enqueue("a", "b", Lane::Data, 0, &format!("e{}", i), 0);
        }
        assert_eq!(dlq.depth(), 3);
        assert_eq!(dlq.total_enqueued(), 5);
    }

    #[test]
    fn test_replay_tracking() {
        let dlq = DeadLetterQueue::new(100);
        dlq.enqueue("a", "b", Lane::Data, 0, "err", 0);
        dlq.mark_replayed(1);
        assert_eq!(dlq.total_replayed(), 1);
    }

    #[test]
    fn test_clear() {
        let dlq = DeadLetterQueue::new(100);
        dlq.enqueue("a", "b", Lane::Data, 0, "err", 0);
        dlq.clear();
        assert_eq!(dlq.depth(), 0);
    }
}

// ==================== Backpressure Tests ====================

#[cfg(test)]
mod backpressure_tests {
    use vil_server_mesh::backpressure::*;

    #[test]
    fn test_initial_state() {
        let ctrl = BackpressureController::new("svc", 100);
        assert!(ctrl.is_accepting());
        assert_eq!(ctrl.in_flight(), 0);
        assert_eq!(ctrl.throttle_rate(), 0);
    }

    #[test]
    fn test_throttle_at_high_watermark() {
        let ctrl = BackpressureController::new("svc", 10);
        // high_watermark = 80% of 10 = 8
        for _ in 0..7 {
            let _ = ctrl.request_enter();
        }
        // 8th request crosses high watermark
        let signal = ctrl.request_enter();
        assert!(matches!(signal, Some(BackpressureSignal::Throttle { .. })));
    }

    #[test]
    fn test_pause_at_max() {
        let ctrl = BackpressureController::new("svc", 5);
        for _ in 0..4 {
            let _ = ctrl.request_enter();
        }
        let signal = ctrl.request_enter(); // 5th = max
        assert!(matches!(signal, Some(BackpressureSignal::Pause)));
        assert!(!ctrl.is_accepting());
    }

    #[test]
    fn test_upstream_throttle() {
        let throttle = UpstreamThrottle::new("downstream");
        assert!(throttle.can_send());

        throttle.apply_signal(&BackpressureSignal::Pause);
        assert!(!throttle.can_send());

        throttle.apply_signal(&BackpressureSignal::Resume);
        assert!(throttle.can_send());
    }

    #[test]
    fn test_signal_serialization() {
        let signal = BackpressureSignal::Throttle { max_rate: 500 };
        let bytes = signal.to_bytes();
        let parsed = BackpressureSignal::from_bytes(&bytes).unwrap();
        assert!(matches!(
            parsed,
            BackpressureSignal::Throttle { max_rate: 500 }
        ));
    }
}

// ==================== Event Bus Tests ====================

#[cfg(test)]
mod event_bus_tests {
    use vil_server_mesh::event_bus::EventBus;

    #[test]
    fn test_publish_subscribe() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe("orders");

        bus.publish("orders", "order-service", b"new order".to_vec());

        // Use try_recv in sync context
        assert_eq!(bus.total_published(), 1);
        assert_eq!(bus.subscriber_count("orders"), 1);
    }

    #[test]
    fn test_topic_count() {
        let bus = EventBus::new(16);
        bus.publish("topic_a", "svc", b"data".to_vec());
        bus.publish("topic_b", "svc", b"data".to_vec());
        assert_eq!(bus.topic_count(), 2);
    }

    #[test]
    fn test_topics_list() {
        let bus = EventBus::new(16);
        let _ = bus.subscribe("alpha");
        let _ = bus.subscribe("beta");
        let topics = bus.topics();
        assert_eq!(topics.len(), 2);
    }
}

// ==================== Load Balancer Tests ====================

#[cfg(test)]
mod load_balancer_tests {
    use vil_server_mesh::load_balancer::*;

    #[test]
    fn test_round_robin() {
        let endpoints = vec![
            LbEndpoint::new("host1:8080"),
            LbEndpoint::new("host2:8080"),
            LbEndpoint::new("host3:8080"),
        ];
        let lb = LoadBalancer::new(endpoints, LbStrategy::RoundRobin);

        assert_eq!(lb.next().unwrap().address, "host1:8080");
        assert_eq!(lb.next().unwrap().address, "host2:8080");
        assert_eq!(lb.next().unwrap().address, "host3:8080");
        assert_eq!(lb.next().unwrap().address, "host1:8080"); // wraps
    }

    #[test]
    fn test_canary_routing() {
        let endpoints = vec![
            LbEndpoint::new("stable:8080"),
            LbEndpoint::new("canary:8080").canary(),
        ];
        let lb = LoadBalancer::new(endpoints, LbStrategy::Canary { canary_weight: 10 });

        let mut canary_count = 0;
        let total = 1000;
        for _ in 0..total {
            if lb.next().unwrap().is_canary {
                canary_count += 1;
            }
        }
        // ~10% should go to canary (±5% tolerance)
        assert!(canary_count > 50);
        assert!(canary_count < 150);
    }

    #[test]
    fn test_empty_endpoints() {
        let lb = LoadBalancer::new(vec![], LbStrategy::RoundRobin);
        assert!(lb.next().is_none());
    }

    #[test]
    fn test_endpoint_count() {
        let endpoints = vec![LbEndpoint::new("a"), LbEndpoint::new("b")];
        let lb = LoadBalancer::new(endpoints, LbStrategy::RoundRobin);
        assert_eq!(lb.endpoint_count(), 2);
    }
}

// ==================== Typed RPC Tests ====================

#[cfg(test)]
mod rpc_tests {
    use serde::{Deserialize, Serialize};
    use vil_server_mesh::typed_rpc::RpcRegistry;

    #[derive(Serialize, Deserialize)]
    struct AddRequest {
        a: i32,
        b: i32,
    }

    #[derive(Serialize, Deserialize)]
    struct AddResponse {
        result: i32,
    }

    #[test]
    fn test_register_and_invoke() {
        let mut registry = RpcRegistry::new();
        registry.register::<AddRequest, AddResponse, _>("add", |req| AddResponse {
            result: req.a + req.b,
        });

        let input = serde_json::to_vec(&AddRequest { a: 3, b: 4 }).unwrap();
        let output = registry.invoke("add", &input).unwrap();
        let resp: AddResponse = serde_json::from_slice(&output).unwrap();
        assert_eq!(resp.result, 7);
    }

    #[test]
    fn test_invoke_not_found() {
        let registry = RpcRegistry::new();
        let result = registry.invoke("nonexistent", b"{}");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_endpoints() {
        let mut registry = RpcRegistry::new();
        registry.register::<AddRequest, AddResponse, _>("add", |req| AddResponse {
            result: req.a + req.b,
        });
        registry.register::<AddRequest, AddResponse, _>("multiply", |req| AddResponse {
            result: req.a * req.b,
        });
        assert_eq!(registry.count(), 2);
        assert_eq!(registry.endpoints().len(), 2);
    }
}
