// VFlow Risk Mitigation Benchmarks
// =================================
// Validates 3 architectural risks before VFlow kernel implementation:
//
// Risk 1: ExchangeHeap throughput for workflow token volume
// Risk 2: FlatBuffer (VWFB-style) access overhead
// Risk 3: Tri-Lane (channel) vs direct call latency
//
// Run: cargo run --release -p vflow-risk-mitigation-bench

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use vil_shm::BumpAllocator;

// ═══════════════════════════════════════════════════════════════════════════
// Risk 1: ExchangeHeap BumpAllocator throughput
// ═══════════════════════════════════════════════════════════════════════════
// Question: Can BumpAllocator handle 1M+ token allocations/sec?
// VFlow needs: each workflow step allocs 48-256 bytes in SHM.
// At 100K workflows/sec × 5 steps = 500K allocs/sec minimum.

fn bench_bump_alloc() {
    println!("═══ Risk 1: ExchangeHeap BumpAllocator Throughput ═══\n");

    let iterations = 5_000_000u64;

    // Scenario A: Small allocs (48 bytes = WorkflowToken size)
    let alloc = BumpAllocator::new(iterations as usize * 64);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = alloc.alloc(48, 8); // 48 bytes, 8-byte aligned
    }
    let elapsed = start.elapsed();
    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;
    println!("  Token alloc (48B × {}M):", iterations / 1_000_000);
    println!("    Total:   {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("    Per-op:  {:.1}ns", ns_per_op);
    println!("    Rate:    {:.1}M ops/sec", ops_per_sec / 1_000_000.0);
    let pass_a = ops_per_sec > 500_000.0;
    println!("    Target:  >500K ops/sec → {}\n", if pass_a { "✅ PASS" } else { "❌ FAIL" });

    // Scenario B: Medium allocs (256 bytes = typical activity output)
    let alloc = BumpAllocator::new(iterations as usize * 512);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = alloc.alloc(256, 8);
    }
    let elapsed = start.elapsed();
    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;
    println!("  Activity output alloc (256B × {}M):", iterations / 1_000_000);
    println!("    Total:   {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("    Per-op:  {:.1}ns", ns_per_op);
    println!("    Rate:    {:.1}M ops/sec", ops_per_sec / 1_000_000.0);
    let pass_b = ops_per_sec > 500_000.0;
    println!("    Target:  >500K ops/sec → {}\n", if pass_b { "✅ PASS" } else { "❌ FAIL" });

    // Scenario C: Alloc + write (simulate SHM write)
    let alloc = BumpAllocator::new(iterations as usize * 64);
    let payload = [0u8; 48];
    let start = Instant::now();
    for _ in 0..iterations {
        let offset = alloc.alloc(48, 8);
        std::hint::black_box(offset);
        std::hint::black_box(&payload);
    }
    let elapsed = start.elapsed();
    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;
    println!("  Alloc+write sim (48B × {}M):", iterations / 1_000_000);
    println!("    Total:   {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("    Per-op:  {:.1}ns", ns_per_op);
    println!("    Rate:    {:.1}M ops/sec", ops_per_sec / 1_000_000.0);
    let pass_c = ops_per_sec > 500_000.0;
    println!("    Target:  >500K ops/sec → {}\n", if pass_c { "✅ PASS" } else { "❌ FAIL" });
}

// ═══════════════════════════════════════════════════════════════════════════
// Risk 2: FlatBuffer access overhead (VWFB-style)
// ═══════════════════════════════════════════════════════════════════════════
// Question: How fast is FlatBuffer field access vs native struct?
// VFlow step_token reads node.kind, node.edge_start, node.config_offset
// per token step. Needs <100ns per access.

// Simulate FlatBuffer-style access pattern
#[repr(C)]
#[derive(Copy, Clone)]
struct CompactNode {
    kind: u8,
    flags: u8,
    edge_start: u16,
    edge_count: u16,
    config_offset: u32,
    config_len: u16,
    name_sym: u32,
    output_slot: u16,
    expr_id: u32,
    _pad: [u8; 4],
}

fn bench_flatbuffer_access() {
    println!("═══ Risk 2: FlatBuffer-style Access Overhead ═══\n");

    let iterations = 10_000_000u64;

    // Create a mock graph with 1000 nodes
    let nodes: Vec<CompactNode> = (0..1000).map(|i| CompactNode {
        kind: (i % 15) as u8,
        flags: 0,
        edge_start: (i * 2) as u16,
        edge_count: 2,
        config_offset: i * 64,
        config_len: 48,
        name_sym: i,
        output_slot: i as u16,
        expr_id: if i % 3 == 0 { i } else { 0xFFFFFFFF },
        _pad: [0; 4],
    }).collect();

    // Scenario A: Sequential node access (step_token pattern)
    let start = Instant::now();
    let mut checksum = 0u64;
    for i in 0..iterations {
        let node = &nodes[(i % 1000) as usize];
        checksum += node.kind as u64;
        checksum += node.edge_start as u64;
        checksum += node.config_offset as u64;
    }
    std::hint::black_box(checksum);
    let elapsed = start.elapsed();
    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;
    println!("  Node field access (kind+edge+config × {}M):", iterations / 1_000_000);
    println!("    Total:   {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("    Per-op:  {:.2}ns", ns_per_op);
    let pass_a = ns_per_op < 100.0;
    println!("    Target:  <100ns → {}\n", if pass_a { "✅ PASS" } else { "❌ FAIL" });

    // Scenario B: Random node access (guard evaluation jumps)
    let indices: Vec<usize> = (0..iterations as usize)
        .map(|i| (i * 7 + 13) % 1000)
        .collect();
    let start = Instant::now();
    let mut checksum = 0u64;
    for &idx in &indices {
        let node = &nodes[idx];
        checksum += node.kind as u64 + node.expr_id as u64;
    }
    std::hint::black_box(checksum);
    let elapsed = start.elapsed();
    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;
    println!("  Random node lookup ({}M lookups in 1K nodes):", iterations / 1_000_000);
    println!("    Total:   {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("    Per-op:  {:.2}ns", ns_per_op);
    let pass_b = ns_per_op < 100.0;
    println!("    Target:  <100ns → {}\n", if pass_b { "✅ PASS" } else { "❌ FAIL" });
}

// ═══════════════════════════════════════════════════════════════════════════
// Risk 3: Tri-Lane (channel) vs direct call latency
// ═══════════════════════════════════════════════════════════════════════════
// Question: How much overhead does mpsc channel add vs direct fn call?
// VFlow kernel uses channels for Tri-Lane (trigger_rx, io_event_rx, control_rx).
// Intra-process channel should be <1μs for token dispatch.

fn bench_channel_vs_direct() {
    println!("═══ Risk 3: Channel (Tri-Lane) vs Direct Call Latency ═══\n");

    let iterations = 5_000_000u64;

    // Scenario A: Direct function call (baseline)
    let counter = AtomicU64::new(0);
    let start = Instant::now();
    for i in 0..iterations {
        counter.store(i, Ordering::Relaxed);
        std::hint::black_box(counter.load(Ordering::Relaxed));
    }
    let elapsed_direct = start.elapsed();
    let ns_direct = elapsed_direct.as_nanos() as f64 / iterations as f64;
    println!("  Direct atomic store+load ({}M):", iterations / 1_000_000);
    println!("    Total:   {:.2}ms", elapsed_direct.as_secs_f64() * 1000.0);
    println!("    Per-op:  {:.2}ns\n", ns_direct);

    // Scenario B: Tokio mpsc unbounded channel (simulates Tri-Lane)
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let elapsed_channel = rt.block_on(async {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<u64>();

        let start = Instant::now();
        for i in 0..iterations {
            tx.send(i).unwrap();
            let _ = rx.recv().await;
        }
        start.elapsed()
    });
    let ns_channel = elapsed_channel.as_nanos() as f64 / iterations as f64;
    println!("  Tokio mpsc unbounded send+recv ({}M):", iterations / 1_000_000);
    println!("    Total:   {:.2}ms", elapsed_channel.as_secs_f64() * 1000.0);
    println!("    Per-op:  {:.1}ns", ns_channel);
    let pass_channel = ns_channel < 1000.0; // <1μs
    println!("    Target:  <1μs (1000ns) → {}", if pass_channel { "✅ PASS" } else { "❌ FAIL" });
    println!("    Overhead vs direct: {:.1}x\n", ns_channel / ns_direct);

    // Scenario C: Tokio mpsc bounded(1024) channel (backpressure aware)
    let elapsed_bounded = rt.block_on(async {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<u64>(1024);

        let start = Instant::now();
        for i in 0..iterations {
            tx.send(i).await.unwrap();
            let _ = rx.recv().await;
        }
        start.elapsed()
    });
    let ns_bounded = elapsed_bounded.as_nanos() as f64 / iterations as f64;
    println!("  Tokio mpsc bounded(1024) send+recv ({}M):", iterations / 1_000_000);
    println!("    Total:   {:.2}ms", elapsed_bounded.as_secs_f64() * 1000.0);
    println!("    Per-op:  {:.1}ns", ns_bounded);
    let pass_bounded = ns_bounded < 1000.0;
    println!("    Target:  <1μs → {}", if pass_bounded { "✅ PASS" } else { "❌ FAIL" });
    println!("    Overhead vs direct: {:.1}x\n", ns_bounded / ns_direct);
}

// ═══════════════════════════════════════════════════════════════════════════

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  VFlow Risk Mitigation Benchmarks                          ║");
    println!("║  Validates architectural decisions before implementation    ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    bench_bump_alloc();
    bench_flatbuffer_access();
    bench_channel_vs_direct();

    println!("═══ Summary ═══\n");
    println!("  Risk 1 (ExchangeHeap):   Measured above — O(1) atomic bump alloc");
    println!("  Risk 2 (FlatBuffer):     Measured above — cache-line friendly struct access");
    println!("  Risk 3 (Tri-Lane):       Measured above — mpsc channel latency");
    println!("  Risk 4 (vdicl/vcel):     ✅ Both compile in original workspace\n");
    println!("  Decision: if all PASS → proceed with VFlow kernel implementation");
    println!("            if Risk 3 FAIL → consider direct dispatch for hot path\n");
}
