use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use vil_types::{CleanupPolicy, Epoch, PortId, ProcessId, RegionId, SampleId};
use vil_shm::ExchangeHeap;

/// Maximum entities in the shared registry.
pub const MAX_PROCESSES: usize = 64;
pub const MAX_PORTS: usize = 256;
pub const MAX_SAMPLES: usize = 1024;
pub const MAX_HOSTS: usize = 16;
pub const MAX_ROUTES: usize = 512;

/// Process record safe for Shared Memory.
#[repr(C)]
pub struct SharedProcessRecord {
    pub active: AtomicBool,
    pub id: ProcessId,
    pub name: [u8; 32],
    pub cleanup: CleanupPolicy,
    pub epoch: AtomicU64,
    pub alive: AtomicBool,
}

/// Port record safe for Shared Memory.
#[repr(C)]
pub struct SharedPortRecord {
    pub active: AtomicBool,
    pub id: PortId,
    pub process_id: ProcessId,
    pub direction: vil_types::PortDirection,
    pub name: [u8; 32],
}

#[repr(C)]
pub struct SharedSampleRecord {
    pub active: AtomicBool,
    pub id: SampleId,
    pub owner: ProcessId,
    pub origin_host: vil_types::HostId,
    pub origin_port: PortId,
    pub published: AtomicBool,
    pub expected_reads: AtomicU32,
    pub completed_reads: AtomicU32,
    pub offset: AtomicU64,
    pub region_id: RegionId,
    pub size: u32,
    pub align: u32,
}

#[repr(C)]
pub struct SharedHostRecord {
    pub active: AtomicBool,
    pub id: vil_types::HostId,
    pub addr: [u8; 64], // e.g. "192.168.1.10:3080"
    pub last_heartbeat: AtomicU64,
}

#[repr(C)]
pub struct SharedRouteRecord {
    pub active: AtomicBool,
    pub from: PortId,
    pub to: PortId,
}

#[repr(C)]
pub struct SharedRegistryLayout {
    pub magic: u64,
    pub next_process_id: AtomicU64,
    pub next_port_id: AtomicU64,
    pub next_sample_id: AtomicU64,
    pub process_table: [SharedProcessRecord; MAX_PROCESSES],
    pub port_table: [SharedPortRecord; MAX_PORTS],
    pub sample_table: [SharedSampleRecord; MAX_SAMPLES],
    pub host_table: [SharedHostRecord; MAX_HOSTS],
    pub route_table: [SharedRouteRecord; MAX_ROUTES],
    
    /// Global performance counters
    pub global_counters: vil_obs::counters::RuntimeCounters,
    /// Global latency tracking
    pub global_latency: vil_obs::latency::LatencyTracker,
}

#[derive(Debug)]
pub struct ProcessSnapshot {
    pub id: ProcessId,
    pub name: String,
    pub alive: bool,
    pub epoch: vil_types::Epoch,
}

#[derive(Debug)]
pub struct PortSnapshot {
    pub id: PortId,
    pub process_id: ProcessId,
    pub direction: vil_types::PortDirection,
    pub name: String,
}

#[derive(Debug)]
pub struct SampleSnapshot {
    pub id: SampleId,
    pub owner: ProcessId,
    pub origin_host: vil_types::HostId,
    pub active: bool,
    pub published: bool,
    pub reads: (u32, u32), // (completed, expected)
    pub offset: u64,
    pub region_id: RegionId,
    pub size: u32,
    pub align: u32,
}

impl SharedRegistryLayout {
    pub const MAGIC: u64 = 0x564c414e47524547; // "VILREG"

    #[doc(alias = "vil_keep")]
    pub fn init(&mut self) {
        self.magic = Self::MAGIC;
        self.next_process_id.store(1, Ordering::SeqCst);
        self.next_port_id.store(1, Ordering::SeqCst);
        self.next_sample_id.store(1, Ordering::SeqCst);
    }
}

/// Manager for the Shared Registry.
#[derive(Clone)]
pub struct ShmRegistry {
    _heap: ExchangeHeap,
    _region_id: vil_types::RegionId,
    layout_ptr: *mut SharedRegistryLayout,
    _local_host_id: vil_types::HostId,
}

unsafe impl Send for ShmRegistry {}
unsafe impl Sync for ShmRegistry {}

impl ShmRegistry {
    /// Create or attach to a shared registry with the default name.
    #[doc(alias = "vil_keep")]
    pub fn new_or_attach(heap: ExchangeHeap, host_id: vil_types::HostId) -> std::io::Result<Self> {
        Self::new_or_attach_with_name(heap, "shared_registry", host_id)
    }

    /// Create or attach to a shared registry with a custom name.
    #[doc(alias = "vil_keep")]
    pub fn new_or_attach_with_name(heap: ExchangeHeap, name: &str, host_id: vil_types::HostId) -> std::io::Result<Self> {
        let size = std::mem::size_of::<SharedRegistryLayout>();
        // Region must be at least PAGE_SIZE so the PagedAllocator has >= 1 page.
        let region_size = size.max(vil_shm::paged_allocator::PAGE_SIZE);

        // Try attaching first
        let region_id = match heap.attach_region(name) {
            Ok(id) => {
                // Check if the existing region is large enough
                let stats = heap.region_stats(id).unwrap();
                if stats.capacity < size {
                    // Too small (old layout). Recreate.
                    let _ = ExchangeHeap::unlink_region(name);
                    heap.create_named_region(name, region_size)?
                } else {
                    // Clear stale sample table from previous process runs.
                    // Samples are per-session and don't survive process restart.
                    let layout_ptr = heap.get_region_ptr(id).unwrap() as *mut SharedRegistryLayout;
                    let layout = unsafe { &mut *layout_ptr };
                    for slot in &mut layout.sample_table {
                        slot.active.store(false, Ordering::Release);
                    }
                    id
                }
            },
            Err(_) => {
                // If attach fails (does not exist), create new
                let id = heap.create_named_region(name, region_size)?;
                // Initialize layout
                let ptr = heap.alloc_bytes(id, size, 8).ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::Other, "Failed to alloc registry layout")
                })?;
                
                // Write magic & initial metadata
                let layout = SharedRegistryLayout {
                    magic: SharedRegistryLayout::MAGIC,
                    next_process_id: AtomicU64::new(1),
                    next_port_id: AtomicU64::new(1),
                    next_sample_id: AtomicU64::new(1),
                    process_table: unsafe { std::mem::zeroed() },
                    port_table: unsafe { std::mem::zeroed() },
                    sample_table: unsafe { std::mem::zeroed() },
                    host_table: unsafe { std::mem::zeroed() },
                    route_table: unsafe { std::mem::zeroed() },
                    global_counters: vil_obs::counters::RuntimeCounters::new(),
                    global_latency: vil_obs::latency::LatencyTracker::new(),
                };
                heap.write_bytes(id, ptr, unsafe {
                    std::slice::from_raw_parts(&layout as *const _ as *const u8, size)
                });
                id
            }
        };

        let layout_ptr = heap.get_region_ptr(region_id).unwrap() as *mut SharedRegistryLayout;
        Ok(Self { _heap: heap, _region_id: region_id, layout_ptr, _local_host_id: host_id })
    }

    // --- Internal Helpers ---

    #[doc(alias = "vil_keep")]
    fn get_layout<F, R>(&self, f: F) -> R 
    where F: FnOnce(&SharedRegistryLayout) -> R 
    {
        let layout = unsafe { &*self.layout_ptr };
        f(layout)
    }

    #[doc(alias = "vil_keep")]
    fn get_layout_mut<F, R>(&self, f: F) -> R 
    where F: FnOnce(&mut SharedRegistryLayout) -> R 
    {
        let layout = unsafe { &mut *self.layout_ptr };
        f(layout)
    }

    // --- API ---

    #[doc(alias = "vil_keep")]
    pub fn register_process(&self, id: ProcessId, name: &str, cleanup: CleanupPolicy) -> bool {
        self.get_layout_mut(|layout| {
            for slot in &mut layout.process_table {
                if !slot.active.load(Ordering::Acquire) {
                    slot.id = id;
                    slot.cleanup = cleanup;
                    slot.epoch.store(1, Ordering::Release);
                    slot.alive.store(true, Ordering::Release);
                    
                    let name_bytes = name.as_bytes();
                    let len = name_bytes.len().min(32);
                    slot.name[..len].copy_from_slice(&name_bytes[..len]);
                    
                    slot.active.store(true, Ordering::Release);
                    return true;
                }
            }
            false
        })
    }

    pub fn mark_process_dead(&self, id: ProcessId) {
        self.get_layout_mut(|layout| {
            for slot in &mut layout.process_table {
                if slot.active.load(Ordering::Acquire) && slot.id == id {
                    slot.alive.store(false, Ordering::Release);
                    break;
                }
            }
        });
    }

    pub fn is_process_alive(&self, id: ProcessId) -> bool {
        self.get_layout_mut(|layout| {
            for slot in &layout.process_table {
                if slot.active.load(Ordering::Acquire) && slot.id == id {
                    return slot.alive.load(Ordering::Acquire);
                }
            }
            false
        })
    }

    // --- ID Allocation ---

    #[doc(alias = "vil_keep")]
    pub fn next_process_id(&self) -> ProcessId {
        self.get_layout_mut(|layout| {
            ProcessId(layout.next_process_id.fetch_add(1, Ordering::SeqCst))
        })
    }

    #[doc(alias = "vil_keep")]
    pub fn next_port_id(&self) -> PortId {
        self.get_layout_mut(|layout| {
            PortId(layout.next_port_id.fetch_add(1, Ordering::SeqCst))
        })
    }

    pub fn next_sample_id(&self) -> SampleId {
        self.get_layout_mut(|layout| {
            SampleId(layout.next_sample_id.fetch_add(1, Ordering::SeqCst))
        })
    }

    // --- Port operations ---

    #[doc(alias = "vil_keep")]
    pub fn register_port(&self, port_id: PortId, process_id: ProcessId, direction: vil_types::PortDirection, name: &str) -> bool {
        self.get_layout_mut(|layout| {
            for slot in &mut layout.port_table {
                if !slot.active.load(Ordering::Acquire) {
                    slot.id = port_id;
                    slot.process_id = process_id;
                    slot.direction = direction;
                    
                    let name_bytes = name.as_bytes();
                    let len = name_bytes.len().min(32);
                    slot.name[..len].copy_from_slice(&name_bytes[..len]);
                    
                    slot.active.store(true, Ordering::Release);
                    return true;
                }
            }
            false
        })
    }

    // --- Sample operations ---

    pub fn register_sample(
        &self,
        sample_id: SampleId,
        owner: ProcessId,
        origin_host: vil_types::HostId,
        origin_port: PortId,
        expected_reads: u32,
        region_id: RegionId,
        offset: u64,
        size: u32,
        align: u32,
    ) -> bool {
        self.get_layout_mut(|layout| {
            for slot in &mut layout.sample_table {
                if !slot.active.load(Ordering::Acquire) {
                    slot.id = sample_id;
                    slot.owner = owner;
                    slot.origin_host = origin_host;
                    slot.origin_port = origin_port;
                    slot.published.store(false, Ordering::Release);
                    slot.expected_reads.store(expected_reads, Ordering::Release);
                    slot.completed_reads.store(0, Ordering::Release);
                    slot.offset.store(offset, Ordering::Release);
                    slot.region_id = region_id;
                    slot.size = size;
                    slot.align = align;
                    
                    slot.active.store(true, Ordering::Release);
                    return true;
                }
            }
            false
        })
    }

    pub fn update_sample_offset(&self, sample_id: SampleId, new_offset: u64) -> bool {
        self.get_layout_mut(|layout| {
            for slot in &mut layout.sample_table {
                if slot.active.load(Ordering::Acquire) && slot.id == sample_id {
                    slot.offset.store(new_offset, Ordering::SeqCst);
                    return true;
                }
            }
            false
        })
    }

    pub fn get_sample_location(&self, sample_id: SampleId) -> Option<(RegionId, u64)> {
        self.get_layout(|layout| {
            for slot in &layout.sample_table {
                if slot.active.load(Ordering::Acquire) && slot.id == sample_id {
                    return Some((slot.region_id, slot.offset.load(Ordering::Acquire)));
                }
            }
            None
        })
    }

    pub fn mark_published(&self, sample_id: SampleId) {
        self.get_layout_mut(|layout| {
            for slot in &mut layout.sample_table {
                if slot.active.load(Ordering::Acquire) && slot.id == sample_id {
                    slot.published.store(true, Ordering::Release);
                    break;
                }
            }
        });
    }

    pub fn mark_received(&self, sample_id: SampleId) {
        // Release the sample slot after recv so the table doesn't fill up.
        // For single-reader pipelines, completed_reads reaches expected_reads
        // after one recv, allowing immediate slot reclamation.
        self.get_layout_mut(|layout| {
            for slot in &mut layout.sample_table {
                if slot.active.load(Ordering::Acquire) && slot.id == sample_id {
                    let completed = slot.completed_reads.fetch_add(1, Ordering::SeqCst) + 1;
                    let expected = slot.expected_reads.load(Ordering::Acquire);
                    if completed >= expected {
                        slot.active.store(false, Ordering::Release);
                    }
                    break;
                }
            }
        });
    }

    /// Mark one read as complete.
    /// Returns true if the sample has been fully read by all targets.
    pub fn mark_release_read(&self, sample_id: SampleId) -> bool {
        self.get_layout_mut(|layout| {
            for slot in &mut layout.sample_table {
                if slot.active.load(Ordering::Acquire) && slot.id == sample_id {
                    let completed = slot.completed_reads.fetch_add(1, Ordering::SeqCst) + 1;
                    let expected = slot.expected_reads.load(Ordering::Acquire);
                    return completed >= expected;
                }
            }
            false
        })
    }

    pub fn reclaim_sample(&self, sample_id: SampleId) {
        self.get_layout_mut(|layout| {
            for slot in &mut layout.sample_table {
                if slot.active.load(Ordering::Acquire) && slot.id == sample_id {
                    slot.active.store(false, Ordering::Release);
                    break;
                }
            }
        });
    }

    // --- Reporting ---

    pub fn sample_count(&self) -> usize {
        self.get_layout_mut(|layout| {
            layout.sample_table.iter()
                .filter(|s| s.active.load(Ordering::Acquire))
                .count()
        })
    }

    pub fn snapshot_processes(&self) -> Vec<ProcessSnapshot> {
        self.get_layout_mut(|layout| {
            layout.process_table.iter()
                .filter(|s| s.active.load(Ordering::Acquire))
                .map(|p| ProcessSnapshot {
                    id: p.id,
                    name: String::from_utf8_lossy(&p.name).trim_matches('\0').to_string(),
                    alive: p.alive.load(Ordering::Relaxed),
                    epoch: Epoch(p.epoch.load(Ordering::Relaxed)),
                })
                .collect()
        })
    }
    pub fn snapshot_ports(&self) -> Vec<PortSnapshot> {
        self.get_layout_mut(|layout| {
            layout.port_table.iter()
                .filter(|s| s.active.load(Ordering::Acquire))
                .map(|p| PortSnapshot {
                    id: p.id,
                    process_id: p.process_id,
                    direction: p.direction,
                    name: String::from_utf8_lossy(&p.name).trim_matches('\0').to_string(),
                })
                .collect()
        })
    }

    #[doc(alias = "vil_keep")]
    pub fn register_host(&self, host_id: vil_types::HostId, addr: &str) -> bool {
        self.get_layout_mut(|layout| {
            // Check if already registered
            for slot in &mut layout.host_table {
                if slot.active.load(Ordering::Acquire) && slot.id == host_id {
                    let addr_bytes = addr.as_bytes();
                    let len = addr_bytes.len().min(64);
                    slot.addr = [0u8; 64];
                    slot.addr[..len].copy_from_slice(&addr_bytes[..len]);
                    return true;
                }
            }
            // Register new
            for slot in &mut layout.host_table {
                if !slot.active.load(Ordering::Acquire) {
                    slot.id = host_id;
                    let addr_bytes = addr.as_bytes();
                    let len = addr_bytes.len().min(64);
                    slot.addr[..len].copy_from_slice(&addr_bytes[..len]);
                    slot.last_heartbeat.store(0, Ordering::Release);
                    slot.active.store(true, Ordering::Release);
                    return true;
                }
            }
            false
        })
    }

    pub fn heartbeat(&self, host_id: vil_types::HostId, now_ns: u64) {
        self.get_layout(|layout| {
            for slot in &layout.host_table {
                if slot.active.load(Ordering::Acquire) && slot.id == host_id {
                    slot.last_heartbeat.store(now_ns, Ordering::Release);
                    break;
                }
            }
        });
    }

    pub fn check_dead_hosts(&self, now_ns: u64, timeout_ns: u64) -> Vec<vil_types::HostId> {
        let mut dead = Vec::new();
        self.get_layout_mut(|layout| {
            for slot in &mut layout.host_table {
                if slot.active.load(Ordering::Acquire) {
                    let last = slot.last_heartbeat.load(Ordering::Acquire);
                    if last > 0 && now_ns.saturating_sub(last) > timeout_ns {
                        dead.push(slot.id);
                        slot.active.store(false, Ordering::Release);
                        // Optional: mark processes on this host as dead
                        for proc_slot in &mut layout.process_table {
                            if proc_slot.active.load(Ordering::Acquire) {
                                // In a real system, we'd need host-to-process mapping
                                // For now, we assume processes might be local or we mark all
                                // but we need a better way to filter by host.
                            }
                        }
                    }
                }
            }
        });
        dead
    }

    #[doc(alias = "vil_keep")]
    pub fn register_route(&self, from: PortId, to: PortId) -> bool {
        self.get_layout_mut(|layout| {
            // Check if already exists
            for slot in &layout.route_table {
                if slot.active.load(Ordering::Acquire) && slot.from == from && slot.to == to {
                    return true;
                }
            }
            // Register new
            for slot in &mut layout.route_table {
                if !slot.active.load(Ordering::Acquire) {
                    slot.from = from;
                    slot.to = to;
                    slot.active.store(true, Ordering::Release);
                    return true;
                }
            }
            false
        })
    }

    pub fn unregister_route(&self, from: PortId, to: PortId) -> bool {
        self.get_layout_mut(|layout| {
            for slot in &mut layout.route_table {
                if slot.active.load(Ordering::Acquire) && slot.from == from && slot.to == to {
                    slot.active.store(false, Ordering::Release);
                    return true;
                }
            }
            false
        })
    }

    pub fn clear_routes(&self, from: PortId) {
        self.get_layout_mut(|layout| {
            for slot in &mut layout.route_table {
                if slot.active.load(Ordering::Acquire) && slot.from == from {
                    slot.active.store(false, Ordering::Release);
                }
            }
        });
    }

    pub fn get_routes(&self, from: PortId) -> Vec<PortId> {
        self.get_layout(|layout| {
            layout.route_table.iter()
                .filter(|s| s.active.load(Ordering::Acquire) && s.from == from)
                .map(|s| s.to)
                .collect()
        })
    }

    pub fn get_host_addr(&self, host_id: vil_types::HostId) -> Option<String> {
        self.get_layout(|layout| {
            for slot in &layout.host_table {
                if slot.active.load(Ordering::Acquire) && slot.id == host_id {
                    let addr = String::from_utf8_lossy(&slot.addr).trim_matches('\0').to_string();
                    return Some(addr);
                }
            }
            None
        })
    }

    pub fn snapshot_samples(&self) -> Vec<SampleSnapshot> {
        self.get_layout(|l| {
            l.sample_table.iter()
                .filter(|s| s.active.load(Ordering::Relaxed))
                .map(|s| SampleSnapshot {
                    id: s.id,
                    owner: s.owner,
                    origin_host: s.origin_host,
                    active: s.active.load(Ordering::Relaxed),
                    published: s.published.load(Ordering::Relaxed),
                    reads: (s.completed_reads.load(Ordering::Relaxed), s.expected_reads.load(Ordering::Relaxed)),
                    offset: s.offset.load(Ordering::Relaxed),
                    region_id: s.region_id,
                    size: s.size,
                    align: s.align,
                })
                .collect()
        })
    }

    /// Synchronize data from a remote node (snapshot-based).
    pub fn sync_from_remote(
        &self,
        processes: &[ProcessSnapshot],
        ports: &[PortSnapshot],
        hosts: &[(vil_types::HostId, String)],
    ) {
        for (id, addr) in hosts {
            self.register_host(*id, addr);
        }
        for p in processes {
            // Use the same ID for cluster-wide consistency
            self.register_process(p.id, &p.name, CleanupPolicy::ReclaimOrphans);
        }
        for p in ports {
            self.register_port(p.id, p.process_id, p.direction, &p.name);
        }
    }

    /// Accessor for global performance counters.
    pub fn global_counters(&self) -> &vil_obs::counters::RuntimeCounters {
        let layout = unsafe { &*self.layout_ptr };
        &layout.global_counters
    }

    /// Accessor for global latency tracking.
    pub fn global_latency(&self) -> &vil_obs::latency::LatencyTracker {
        let layout = unsafe { &*self.layout_ptr };
        &layout.global_latency
    }
}

impl vil_shm::DefragRegistry for ShmRegistry {
    fn get_active_samples(&self, region_id: vil_types::RegionId) -> Vec<vil_shm::DefragSample> {
        self.snapshot_samples()
            .into_iter()
            .filter(|s| s.active && s.region_id == region_id)
            .map(|s| vil_shm::DefragSample {
                id: s.id,
                offset: s.offset,
                size: s.size as usize,
                align: s.align as usize,
            })
            .collect()
    }

    fn update_offset(&self, sample_id: vil_types::SampleId, new_offset: u64) {
        self.update_sample_offset(sample_id, new_offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_shm::ExchangeHeap;
    use vil_types::*;

    #[test]
    fn test_shm_registry_cross_instance() {
        let heap = ExchangeHeap::new();
        let host_id = HostId(1);
        let reg1 = ShmRegistry::new_or_attach_with_name(heap.clone(), "test_registry", host_id).unwrap();

        // Register process in instance 1
        let pid = ProcessId(101);
        reg1.register_process(pid, "test_proc", CleanupPolicy::ReclaimOrphans);

        // Instance 2: Attach to same SHM
        let reg2 = ShmRegistry::new_or_attach_with_name(heap.clone(), "test_registry", host_id).unwrap();
        
        // Verify process visibility in instance 2
        assert!(reg2.is_process_alive(pid));

        // Instance 2: Register port
        let port_id = PortId(202);
        reg2.register_port(port_id, pid, vil_types::PortDirection::Out, "test_port");

        // Instance 1: Register sample
        let sid = SampleId(303);
        reg1.register_sample(sid, pid, host_id, port_id, 1, RegionId(0), 0, 1024, 8);
        assert_eq!(reg1.sample_count(), 1);

        // Instance 2: Mark read and reclaim
        assert!(reg2.mark_release_read(sid)); // 1/1 reads
        reg2.reclaim_sample(sid);
        
        // Final verify
        assert_eq!(reg1.sample_count(), 0);
    }
}
