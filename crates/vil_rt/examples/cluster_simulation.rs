use vil_rt::VastarRuntimeWorld;
use vil_types::{
    BackpressurePolicy, BoundaryKind, CleanupPolicy, DeliveryGuarantee, Descriptor, ExecClass,
    GenericToken, HostId, ObservabilitySpec, PortDirection, PortSpec, Priority, ProcessSpec,
    QueueKind, TransferMode, VSlice,
};

static CONS_PORTS: &[PortSpec] = &[PortSpec {
    name: "metrics_in",
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
    println!("🚀 Starting Phase 7 Cross-Host Simulation (Injection Mode)...");

    // 1. Setup Local Runtime representing "Host 2"
    let host_id_2 = HostId(2);
    let runtime = VastarRuntimeWorld::new_shared_with_host(host_id_2).expect("Runtime failed");

    // 2. Setup "Host 1" (Remote) entry in registry
    if let Some(reg) = runtime.shm_registry() {
        reg.register_host(HostId(1), "192.168.1.10:3080");
        reg.register_host(HostId(2), "127.0.0.1:3082");
    }

    // 3. Define local consumer on Host 2
    let cons_spec = ProcessSpec {
        id: "consumer_h2",
        name: "Consumer on Host 2",
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

    let cons_handle = runtime
        .register_process(cons_spec)
        .expect("Failed to register cons");
    let cons_port = cons_handle.port_id("metrics_in").unwrap();

    // 4. Create a "Remote Sample" on Host 1
    let remote_sample_id = vil_types::SampleId(1337);
    let remote_host_id = HostId(1);

    runtime.shm_registry().unwrap().register_sample(
        remote_sample_id,
        vil_types::ProcessId(999),
        remote_host_id,
        vil_types::PortId(888),
        1,
        vil_types::RegionId(0),
        0,
        64,
        8,
    );
    runtime
        .shm_registry()
        .unwrap()
        .mark_published(remote_sample_id);

    // SIMULATE DATA ARRIVAL AT HOST 1 and PULL TO HOST 2
    // Use GenericToken consistently
    let token = GenericToken {
        session_id: 12345,
        is_done: false,
        data: VSlice::from_vec(vec![1, 2, 3, 4, 5]),
    };
    runtime.simulate_pull_completion(remote_sample_id, token);

    println!(
        "📡 Simulated remote sample {} created on Host {}",
        remote_sample_id, remote_host_id
    );

    // 5. MANUALLY INJECT Descriptor from Host 1 into Host 2's Queue
    let descriptor = Descriptor {
        sample_id: remote_sample_id,
        origin_host: remote_host_id,
        origin_port: vil_types::PortId(888),
        lineage_id: 100,
        publish_ts: 1000,
    };

    println!("🔌 Injecting remote descriptor into local consumer queue...");
    runtime
        .inject_descriptor(cons_port, descriptor)
        .expect("Injection failed");

    // 6. Receive on Host 2 (Remote Pull Simulation)
    println!("📥 Receiving on Host 2 (Triggering Host-Aware Logical Path)...");

    // We'll use the EXACT same GenericToken type re-exported by vil_types
    match runtime.recv::<vil_types::GenericToken>(cons_port) {
        Ok(recv_sample) => {
            println!(
                "✨ Received session_id: {} from Host {} (via simulated pull)",
                recv_sample.session_id, remote_host_id
            );
        }
        Err(e) => {
            println!("❌ Recv failed: {:?}", e);
            // If it's a TypeMismatch, print more info
            if let vil_rt::error::RtError::TypeMismatch(t) = e {
                println!("⚠️ Type mismatch details: expected type name is {}", t);
            }
            std::process::exit(1);
        }
    }

    // 7. Verify Counters
    let snapshot = runtime.counters_snapshot();
    println!("📊 Host 2 Stats: Net Pulls = {}", snapshot.net_pulls);

    if snapshot.net_pulls > 0 {
        println!("🏆 SUCCESS: Cross-host RDMA pull verified!");
    } else {
        println!(
            "❌ FAILURE: No net pull detected. Host ID 2 should be distinct from Remote ID 1."
        );
        std::process::exit(1);
    }
}
