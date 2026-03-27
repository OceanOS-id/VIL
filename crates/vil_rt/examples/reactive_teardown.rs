use vil_types::{ProcessSpec, PortSpec, PortDirection, QueueKind, HostId, MessageContract, ExecClass, CleanupPolicy, ObservabilitySpec, BoundaryKind, MessageMeta, LayoutProfile, TransferMode, ControlSignal};
use vil_rt::VastarRuntimeWorld;
use std::time::Duration;

#[derive(Clone, Debug)]
struct DataPoint {
    id: u64,
    val: f64,
}

impl MessageContract for DataPoint {
    const META: MessageMeta = MessageMeta {
        name: "DataPoint",
        layout: LayoutProfile::Flat,
        transfer_caps: &[TransferMode::LoanWrite, TransferMode::LoanRead],
        is_stable: true,
        semantic_kind: vil_types::SemanticKind::Message,
        memory_class: vil_types::MemoryClass::PagedExchange,
    };
}

fn main() -> std::io::Result<()> {
    println!("🚀 Starting Phase 11: Reactive Tri-Lane Orchestration Verification...");

    let world = VastarRuntimeWorld::new_shared_with_host(HostId(1))?;
    
    // --- Node A: Producer ---
    static PORTS_A: &[PortSpec] = &[
        PortSpec { name: "data_out", direction: PortDirection::Out, queue: QueueKind::Spsc, capacity: 1024, backpressure: vil_types::BackpressurePolicy::Block, transfer_mode: TransferMode::LoanWrite, boundary: BoundaryKind::IntraProcess, timeout_ms: None, priority: vil_types::Priority::Normal, delivery: vil_types::DeliveryGuarantee::BestEffort, observability: ObservabilitySpec { tracing: true, metrics: true, lineage: true, audit_sample_handoff: false, latency_class: vil_types::LatencyClass::Normal } },
        PortSpec { name: "ctrl_out", direction: PortDirection::Out, queue: QueueKind::Spsc, capacity: 10, backpressure: vil_types::BackpressurePolicy::Block, transfer_mode: TransferMode::LoanWrite, boundary: BoundaryKind::IntraProcess, timeout_ms: None, priority: vil_types::Priority::High, delivery: vil_types::DeliveryGuarantee::AtLeastOnce, observability: ObservabilitySpec { tracing: true, metrics: true, lineage: true, audit_sample_handoff: false, latency_class: vil_types::LatencyClass::Normal } },
    ];
    let proc_a = world.register_process(ProcessSpec { id: "node_a", name: "Node A", exec: ExecClass::Thread, cleanup: CleanupPolicy::ReclaimOrphans, ports: PORTS_A, observability: ObservabilitySpec { tracing: true, metrics: true, lineage: true, audit_sample_handoff: false, latency_class: vil_types::LatencyClass::Normal } }).unwrap();

    // --- Node B: Transformer ---
    static PORTS_B: &[PortSpec] = &[
        PortSpec { name: "data_in", direction: PortDirection::In, queue: QueueKind::Spsc, capacity: 1024, backpressure: vil_types::BackpressurePolicy::Block, transfer_mode: TransferMode::LoanRead, boundary: BoundaryKind::IntraProcess, timeout_ms: None, priority: vil_types::Priority::Normal, delivery: vil_types::DeliveryGuarantee::BestEffort, observability: ObservabilitySpec { tracing: true, metrics: true, lineage: true, audit_sample_handoff: false, latency_class: vil_types::LatencyClass::Normal } },
        PortSpec { name: "ctrl_in", direction: PortDirection::In, queue: QueueKind::Spsc, capacity: 10, backpressure: vil_types::BackpressurePolicy::Block, transfer_mode: TransferMode::LoanRead, boundary: BoundaryKind::IntraProcess, timeout_ms: None, priority: vil_types::Priority::High, delivery: vil_types::DeliveryGuarantee::AtLeastOnce, observability: ObservabilitySpec { tracing: true, metrics: true, lineage: true, audit_sample_handoff: false, latency_class: vil_types::LatencyClass::Normal } },
        PortSpec { name: "data_out", direction: PortDirection::Out, queue: QueueKind::Spsc, capacity: 1024, backpressure: vil_types::BackpressurePolicy::Block, transfer_mode: TransferMode::LoanWrite, boundary: BoundaryKind::IntraProcess, timeout_ms: None, priority: vil_types::Priority::Normal, delivery: vil_types::DeliveryGuarantee::BestEffort, observability: ObservabilitySpec { tracing: true, metrics: true, lineage: true, audit_sample_handoff: false, latency_class: vil_types::LatencyClass::Normal } },
        PortSpec { name: "ctrl_out", direction: PortDirection::Out, queue: QueueKind::Spsc, capacity: 10, backpressure: vil_types::BackpressurePolicy::Block, transfer_mode: TransferMode::LoanWrite, boundary: BoundaryKind::IntraProcess, timeout_ms: None, priority: vil_types::Priority::High, delivery: vil_types::DeliveryGuarantee::AtLeastOnce, observability: ObservabilitySpec { tracing: true, metrics: true, lineage: true, audit_sample_handoff: false, latency_class: vil_types::LatencyClass::Normal } },
    ];
    let proc_b = world.register_process(ProcessSpec { id: "node_b", name: "Node B", exec: ExecClass::Thread, cleanup: CleanupPolicy::ReclaimOrphans, ports: PORTS_B, observability: ObservabilitySpec { tracing: true, metrics: true, lineage: true, audit_sample_handoff: false, latency_class: vil_types::LatencyClass::Normal } }).unwrap();

    // --- Node C: Consumer ---
    static PORTS_C: &[PortSpec] = &[
        PortSpec { name: "data_in", direction: PortDirection::In, queue: QueueKind::Spsc, capacity: 1024, backpressure: vil_types::BackpressurePolicy::Block, transfer_mode: TransferMode::LoanRead, boundary: BoundaryKind::IntraProcess, timeout_ms: None, priority: vil_types::Priority::Normal, delivery: vil_types::DeliveryGuarantee::BestEffort, observability: ObservabilitySpec { tracing: true, metrics: true, lineage: true, audit_sample_handoff: false, latency_class: vil_types::LatencyClass::Normal } },
        PortSpec { name: "ctrl_in", direction: PortDirection::In, queue: QueueKind::Spsc, capacity: 10, backpressure: vil_types::BackpressurePolicy::Block, transfer_mode: TransferMode::LoanRead, boundary: BoundaryKind::IntraProcess, timeout_ms: None, priority: vil_types::Priority::High, delivery: vil_types::DeliveryGuarantee::AtLeastOnce, observability: ObservabilitySpec { tracing: true, metrics: true, lineage: true, audit_sample_handoff: false, latency_class: vil_types::LatencyClass::Normal } },
    ];
    let proc_c = world.register_process(ProcessSpec { id: "node_c", name: "Node C", exec: ExecClass::Thread, cleanup: CleanupPolicy::ReclaimOrphans, ports: PORTS_C, observability: ObservabilitySpec { tracing: true, metrics: true, lineage: true, audit_sample_handoff: false, latency_class: vil_types::LatencyClass::Normal } }).unwrap();

    // --- Wiring ---
    world.connect(proc_a.port_id("data_out").unwrap(), proc_b.port_id("data_in").unwrap());
    world.connect(proc_a.port_id("ctrl_out").unwrap(), proc_b.port_id("ctrl_in").unwrap());
    world.connect(proc_b.port_id("data_out").unwrap(), proc_c.port_id("data_in").unwrap());
    world.connect(proc_b.port_id("ctrl_out").unwrap(), proc_c.port_id("ctrl_in").unwrap());

    println!("✅ Pipeline wired: Node A -> Node B -> Node C");

    // --- Node A: Push data then DONE ---
    println!("📡 Node A publishing 5 data points...");
    for i in 0..5 {
        world.publish_value(proc_a.id(), proc_a.port_id("data_out").unwrap(), DataPoint { id: i, val: i as f64 * 1.1 }).unwrap();
    }
    println!("🏁 Node A publishing DONE signal for session 42...");
    world.publish_control(proc_a.id(), proc_a.port_id("ctrl_out").unwrap(), ControlSignal::done(42)).unwrap();

    // --- Node B: Transform loop ---
    println!("🔄 Node B processing data...");
    let mut count_b = 0;
    loop {
        if let Ok(data) = world.recv::<DataPoint>(proc_b.port_id("data_in").unwrap()) {
            count_b += 1;
            // Transparently pass data to C
            world.publish_value(proc_b.id(), proc_b.port_id("data_out").unwrap(), data.clone()).unwrap();
        } else if let Ok(ctrl) = world.recv_control(proc_b.port_id("ctrl_in").unwrap()) {
            if let ControlSignal::Done { session_id } = ctrl {
                println!("✅ Node B received DONE for session {}, finished {} samples. Propagating...", session_id, count_b);
                world.publish_control(proc_b.id(), proc_b.port_id("ctrl_out").unwrap(), ControlSignal::done(session_id)).unwrap();
                break;
            }
        } else {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    // --- Node C: Final consumption ---
    println!("📥 Node C finalizing...");
    let mut count_c = 0;
    loop {
        if let Ok(_data) = world.recv::<DataPoint>(proc_c.port_id("data_in").unwrap()) {
            count_c += 1;
        } else if let Ok(ctrl) = world.recv_control(proc_c.port_id("ctrl_in").unwrap()) {
            if let ControlSignal::Done { session_id } = ctrl {
                println!("🏆 Node C received DONE for session {}, total samples processed: {}", session_id, count_c);
                break;
            }
        } else {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    println!("✨ Phase 11 Orchestration SUCCESS!");
    Ok(())
}
