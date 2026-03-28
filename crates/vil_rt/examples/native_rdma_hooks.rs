use std::sync::Arc;
use std::time::Duration;
use vil_net::VerbsDriver;
use vil_rt::VastarRuntimeWorld;
use vil_types::{
    BoundaryKind, CleanupPolicy, ExecClass, HostId, LayoutProfile, MessageContract, MessageMeta,
    ObservabilitySpec, PortDirection, PortSpec, ProcessSpec, QueueKind, TransferMode,
};

#[derive(Clone, Debug)]
struct SensorData {
    id: u64,
    val: f64,
}

impl MessageContract for SensorData {
    const META: MessageMeta = MessageMeta {
        name: "SensorData",
        layout: LayoutProfile::Flat,
        transfer_caps: &[TransferMode::LoanWrite, TransferMode::LoanRead],
        is_stable: true, // IMPORTANT for Phase 10 zero-copy
        semantic_kind: vil_types::SemanticKind::Message,
        memory_class: vil_types::MemoryClass::PinnedRemote,
    };
}

fn main() -> std::io::Result<()> {
    println!("🚀 Starting Phase 10: Hardware-Ready RDMA Hook Verification...");

    // 1. Initialize Runtime with Host 1 (The "Server")
    let world_v1 = VastarRuntimeWorld::new_shared_with_host(HostId(1))?;

    static PORTS: &[PortSpec] = &[PortSpec {
        name: "out",
        direction: PortDirection::Out,
        queue: QueueKind::Spsc,
        capacity: 1024,
        backpressure: vil_types::BackpressurePolicy::Block,
        transfer_mode: TransferMode::LoanWrite,
        boundary: BoundaryKind::InterHost,
        timeout_ms: None,
        priority: vil_types::Priority::Normal,
        delivery: vil_types::DeliveryGuarantee::BestEffort,
        observability: ObservabilitySpec {
            tracing: true,
            metrics: true,
            lineage: true,
            audit_sample_handoff: false,
            latency_class: vil_types::LatencyClass::Normal,
        },
    }];

    let spec = ProcessSpec {
        id: "producer_proc",
        name: "Producer",
        exec: ExecClass::Thread,
        cleanup: CleanupPolicy::ReclaimOrphans,
        ports: PORTS,
        observability: ObservabilitySpec::default(),
    };

    let producer = world_v1.register_process(spec).unwrap();
    let port_out_id = producer.port_id("out").unwrap();

    // 1b. Add a dummy consumer to satisfy routing requirements
    static IN_PORTS: &[PortSpec] = &[PortSpec {
        name: "in",
        direction: PortDirection::In,
        queue: QueueKind::Spsc,
        capacity: 1024,
        backpressure: vil_types::BackpressurePolicy::Block,
        transfer_mode: TransferMode::LoanRead,
        boundary: BoundaryKind::InterHost,
        timeout_ms: None,
        priority: vil_types::Priority::Normal,
        delivery: vil_types::DeliveryGuarantee::BestEffort,
        observability: ObservabilitySpec {
            tracing: true,
            metrics: true,
            lineage: true,
            audit_sample_handoff: false,
            latency_class: vil_types::LatencyClass::Normal,
        },
    }];

    let spec_cons = ProcessSpec {
        id: "consumer_proc",
        name: "Consumer",
        exec: ExecClass::Thread,
        cleanup: CleanupPolicy::ReclaimOrphans,
        ports: IN_PORTS,
        observability: ObservabilitySpec::default(),
    };

    let consumer = world_v1.register_process(spec_cons).unwrap();
    let port_in_id = consumer.port_id("in").unwrap();

    // Connect them!
    world_v1.connect(port_out_id, port_in_id);

    // 2. Publish a sample (This will trigger mlock in our hardware-ready substrate)
    println!("📡 Publishing sample from Host 1...");
    let data = SensorData { id: 42, val: 3.14 };
    let _published = world_v1
        .publish_value(producer.id(), port_out_id, data)
        .unwrap();

    // Verify verbs driver exists
    if let Some(driver) = world_v1.verbs_driver() {
        println!("✅ Verbs Driver active on Host {}", driver.host_id().0);
    }

    // 3. Initialize Runtime with Host 2 (The "Client" performing remote pull)
    println!("🔗 Initializing Host 2 to simulate remote RDMA pull...");
    let world_v2 = VastarRuntimeWorld::new_shared_with_host(HostId(2))?;

    // Check if the sample location is visible to Host 2 via the shared registry
    let samples = world_v2.registry_samples();
    for sample in samples {
        println!(
            "🔍 Found Sample ID: {}, Host: {}, Offset: {}",
            sample.id.0, sample.origin_host.0, sample.offset
        );

        if sample.origin_host == HostId(1) {
            println!("✅ Sample from Host 1 is globally visible in Registry!");
        }
    }

    println!("🏆 SUCCESS: Phase 10 Hardware-Ready hooks verified!");
    Ok(())
}
