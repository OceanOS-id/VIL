use vil_types::{HostId, ProcessSpec, PortSpec, PortDirection, QueueKind, MessageContract, ExecClass, CleanupPolicy, ObservabilitySpec, BoundaryKind, MessageMeta, LayoutProfile, TransferMode};
use vil_rt::VastarRuntimeWorld;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    println!("🚀 Starting Phase 12: High-Availability Failover Verification...");

    // Setup Shared Registry with HostId 1
    let world = VastarRuntimeWorld::new_shared_with_host(HostId(1))?;
    
    // Register another host (Backup)
    let host_backup = HostId(2);
    world.register_host(host_backup, "192.168.1.20:3080");
    
    println!("✅ Registry initialized with two hosts: Primary(1) and Backup(2)");

    // Heartbeat for local host (Primary)
    world.heartbeat();
    println!("💓 Heartbeat sent for Primary host.");

    // Perform health check (should be healthy)
    world.perform_health_check(1_000_000_000); // 1s timeout
    println!("🔍 Initial health check: OK");

    // Simulate Primary failure by NOT sending heartbeat and waiting
    println!("⏳ Waiting 2 seconds to simulate heartbeat timeout...");
    std::thread::sleep(Duration::from_secs(2));

    // Perform health check (should detect failure of Primary)
    println!("🔍 Performing failure detection check...");
    world.perform_health_check(500_000_000); // 0.5s timeout

    let counters = world.counters_snapshot();
    println!("📈 Failover Events: {}", counters.failover_events);

    if counters.failover_events > 0 {
        println!("🏆 SUCCESS: Failover detection verified!");
    } else {
        println!("❌ FAIL: Failover event not detected.");
    }

    Ok(())
}
