use vil_rt::VastarRuntimeWorld;
use vil_types::{
    BackpressurePolicy, BoundaryKind, CleanupPolicy, DeliveryGuarantee, ExecClass, GenericToken,
    ObservabilitySpec, PortDirection, PortSpec, Priority, ProcessSpec, QueueKind, TransferMode,
    VSlice,
};

static PROD_PORTS: &[PortSpec] = &[PortSpec {
    name: "data_out",
    direction: PortDirection::Out,
    queue: QueueKind::Spsc,
    capacity: 1024,
    backpressure: BackpressurePolicy::Block,
    transfer_mode: TransferMode::LoanWrite,
    boundary: BoundaryKind::InterThreadLocal,
    timeout_ms: None,
    priority: Priority::Normal,
    delivery: DeliveryGuarantee::BestEffort,
    observability: ObservabilitySpec {
        tracing: true,
        metrics: true,
        lineage: true,
        audit_sample_handoff: false,
        latency_class: vil_types::LatencyClass::Normal,
    },
}];

static CONS_PORTS: &[PortSpec] = &[PortSpec {
    name: "data_in",
    direction: PortDirection::In,
    queue: QueueKind::Spsc,
    capacity: 1024,
    backpressure: BackpressurePolicy::Block,
    transfer_mode: TransferMode::LoanWrite,
    boundary: BoundaryKind::InterThreadLocal,
    timeout_ms: None,
    priority: Priority::Normal,
    delivery: DeliveryGuarantee::BestEffort,
    observability: ObservabilitySpec {
        tracing: true,
        metrics: true,
        lineage: true,
        audit_sample_handoff: false,
        latency_class: vil_types::LatencyClass::Normal,
    },
}];

fn main() {
    println!("🚀 Starting Phase 8 Dynamic Re-routing Verification...");

    // 1. Setup Runtime (Shared Mode for simulation of global visibility)
    let runtime = VastarRuntimeWorld::new_shared().expect("Runtime failed");

    // 2. Register Producer
    let prod_spec = ProcessSpec {
        id: "producer",
        name: "Producer Node",
        exec: ExecClass::Thread,
        cleanup: CleanupPolicy::ReclaimOrphans,
        ports: PROD_PORTS,
        observability: ObservabilitySpec {
            tracing: true,
            metrics: true,
            lineage: true,
            audit_sample_handoff: false,
            latency_class: vil_types::LatencyClass::Normal,
        },
    };
    let prod_handle = runtime.register_process(prod_spec).unwrap();
    let prod_port = prod_handle.port_id("data_out").unwrap();

    // 3. Register Consumer A
    let cons_a_spec = ProcessSpec {
        id: "consumer_a",
        name: "Queue Consumer A",
        exec: ExecClass::Thread,
        cleanup: CleanupPolicy::ReclaimOrphans,
        ports: CONS_PORTS,
        observability: ObservabilitySpec {
            tracing: true,
            metrics: true,
            lineage: true,
            audit_sample_handoff: false,
            latency_class: vil_types::LatencyClass::Normal,
        },
    };
    let cons_a_handle = runtime.register_process(cons_a_spec).unwrap();
    let cons_a_port = cons_a_handle.port_id("data_in").unwrap();

    // 4. Register Consumer B
    let cons_b_spec = ProcessSpec {
        id: "consumer_b",
        name: "Queue Consumer B",
        exec: ExecClass::Thread,
        cleanup: CleanupPolicy::ReclaimOrphans,
        ports: CONS_PORTS,
        observability: ObservabilitySpec {
            tracing: true,
            metrics: true,
            lineage: true,
            audit_sample_handoff: false,
            latency_class: vil_types::LatencyClass::Normal,
        },
    };
    let cons_b_handle = runtime.register_process(cons_b_spec).unwrap();
    let cons_b_port = cons_b_handle.port_id("data_in").unwrap();

    // 5. Connect P -> C-A (Initial Topology)
    runtime.connect(prod_port, cons_a_port);
    println!("✅ Initial Topology: Producer -> Consumer A");

    // 6. Publish 3 samples to A
    for i in 0..3 {
        let token = GenericToken {
            session_id: i as u64,
            is_done: false,
            data: VSlice::from_vec(vec![1, 2, 3]),
        };
        runtime
            .publish_value(prod_handle.id(), prod_port, token)
            .unwrap();
    }
    println!("📡 Sent 3 samples to A");

    // 7. Verify A received 3, B received 0
    for i in 0..3 {
        let _ = runtime
            .recv::<GenericToken>(cons_a_port)
            .expect("A should have samples");
        println!("  📥 A received sample {}", i);
    }

    match runtime.recv::<GenericToken>(cons_b_port) {
        Err(vil_rt::error::RtError::QueueEmpty(_)) => println!("  ✅ B is empty as expected"),
        _ => panic!("B should have no samples!"),
    }

    // 8. DYNAMIC REROUTE: Move P to B
    println!("🔄 REROUTING: Producer -> Consumer B");
    runtime.reroute(prod_port, vec![cons_b_port]);

    // 9. Publish 3 more samples
    for i in 3..6 {
        let token = GenericToken {
            session_id: i as u64,
            is_done: false,
            data: VSlice::from_vec(vec![4, 5, 6]),
        };
        runtime
            .publish_value(prod_handle.id(), prod_port, token)
            .unwrap();
    }
    println!("📡 Sent 3 samples after reroute");

    // 10. Verify B received 3, A received 0
    for i in 3..6 {
        let _ = runtime
            .recv::<GenericToken>(cons_b_port)
            .expect("B should have samples");
        println!("  📥 B received sample {}", i);
    }

    match runtime.recv::<GenericToken>(cons_a_port) {
        Err(vil_rt::error::RtError::QueueEmpty(_)) => println!("  ✅ A is empty after reroute"),
        _ => panic!("A should have no new samples!"),
    }

    println!("🏆 SUCCESS: Dynamic re-routing verified end-to-end!");
}
