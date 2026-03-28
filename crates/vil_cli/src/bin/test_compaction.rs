use vil_rt::world::VastarRuntimeWorld;
use vil_types::{MessageContract, PortDirection, PortSpec, ProcessSpec, TransferMode};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct LargeData {
    id: u64,
    payload: [u8; 1024], // 1KB
}

impl MessageContract for LargeData {
    const META: vil_types::MessageMeta = vil_types::MessageMeta {
        name: "LargeData",
        layout: vil_types::LayoutProfile::Flat,
        transfer_caps: &[TransferMode::LoanWrite],
        is_stable: true,
        semantic_kind: vil_types::SemanticKind::Message,
        memory_class: vil_types::MemoryClass::PagedExchange,
    };
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Starting Adaptive Compaction Soak Test...");

    let world = VastarRuntimeWorld::new_shared()?;

    let spec = ProcessSpec {
        id: "compactor_test",
        name: "Compactor Test Process",
        exec: vil_types::ExecClass::PinnedWorker,
        cleanup: vil_types::CleanupPolicy::ReclaimOrphans,
        ports: Box::leak(Box::new([
            PortSpec {
                name: "data_lane",
                direction: PortDirection::Out,
                capacity: 1024,
                ..Default::default()
            },
            PortSpec {
                name: "sink_lane",
                direction: PortDirection::In,
                capacity: 1024,
                ..Default::default()
            },
        ])),
        observability: Default::default(),
    };

    let handle = world.register_process(spec)?;
    let p_id = handle.id();
    let port_out = handle.port_id("data_lane")?;
    let port_in = handle.port_id("sink_lane")?;

    // Connect to self for test
    world.connect(port_out, port_in);

    // 1. Fragment the heap
    println!(
        "📦 Step 1: Fragmenting heap (Allocating 500 samples, releasing even-numbered ones)..."
    );
    let mut live_samples = Vec::new();

    for i in 0..500 {
        let data = LargeData {
            id: i,
            payload: [i as u8; 1024],
        };
        let _published = world.publish_value(p_id, port_out, data)?;

        if i % 2 == 1 {
            // Keep odd-numbered samples alive
            let guard = world.recv::<LargeData>(port_in)?;
            live_samples.push(guard);
        } else {
            // Release even-numbered samples immediately to create gaps
            let _guard = world.recv::<LargeData>(port_in)?;
        }
    }

    let stats_before = world.shm_stats();
    println!("📊 SHM Stats Before Compaction: {:?}", stats_before[0]);

    // 2. Verify odd samples are readable
    for guard in &live_samples {
        if guard.id % 2 != 1 {
            panic!("Unexpected sample ID: {}", guard.id);
        }
    }
    println!(
        "✅ Verified {} active samples are readable.",
        live_samples.len()
    );

    // 3. Trigger Compaction
    println!("🧹 Step 2: Triggering Compaction...");
    let moved = world.compact_shm().map_err(|e| e.to_string())?;
    println!("✨ Compaction complete. Moved {} samples.", moved);

    let stats_after = world.shm_stats();
    println!("📊 SHM Stats After Compaction: {:?}", stats_after[0]);

    // 4. Verify odd samples are STILL readable (Lazy Resolution test)
    println!("🔍 Step 3: Verifying samples after relocation (Lazy Resolution)...");
    for guard in &live_samples {
        let val = guard.get();
        if val.id % 2 != 1 || val.payload[0] != (val.id as u8) {
            panic!(
                "DATA CORRUPTION! Sample ID {} is invalid after compaction.",
                val.id
            );
        }
    }
    println!("✅ SUCCESS: All live samples correctly resolved to new offsets.");

    // 5. Verify we can still allocate
    println!("🚀 Step 4: Verifying new allocations in reclaimed space...");
    for i in 1000..1100 {
        let data = LargeData {
            id: i,
            payload: [0xAA; 1024],
        };
        let _p = world.publish_value(p_id, port_out, data)?;
        let _g = world.recv::<LargeData>(port_in)?;
    }
    println!("✅ SUCCESS: New allocations processed successfully.");

    println!("🏁 Compaction Soak Test PASSED!");
    Ok(())
}
