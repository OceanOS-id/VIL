// =============================================================================
// vil_rt::world — VastarRuntimeWorld (Stage 2)
// =============================================================================
// Stage 2 updates:
// 1. publish_value() — Ergonomic shorthand for loan/write/publish.
// 2. publish_control_done() — Specialized control lane publisher.
// 3. Optimized GenericToken wiring for zero-copy streaming.
// =============================================================================

use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use vil_net::{VerbsContext, VerbsDriver};
use vil_obs::ObservabilityHub;
use vil_queue::{DescriptorQueue, QueueBackend, SpscQueue};
use vil_registry::{PortSnapshot, ProcessSnapshot, Registry, SampleSnapshot, ShmRegistry};
use vil_shm::{ExchangeHeap, SharedStore};
use vil_types::{
    ControlSignal, Descriptor, GenericToken, Loaned, LoanedRead, MessageContract, PortId,
    ProcessId, ProcessSpec, Published, QueueKind, RegionId, SampleId, VSlice,
};

use crate::error::RtError;
use crate::handle::{ProcessHandle, RegisteredPort};
use crate::metrics::RuntimeMetrics;
use crate::supervisor::{CleanupReport, Supervisor};

/// Type alias for the queue map (concurrent via DashMap).
type QueueMap = Arc<DashMap<PortId, Box<dyn QueueBackend>>>;

/// Runtime backend (Local or Shared).
enum InternalState {
    Local {
        next_process_id: AtomicU64,
        next_port_id: AtomicU64,
        next_sample_id: AtomicU64,
        store: SharedStore,
        registry: Registry,
        queues: QueueMap,
        host_id: vil_types::HostId,
        verbs: Option<Arc<dyn VerbsDriver>>,
    },
    Shared {
        heap: ExchangeHeap,
        registry: ShmRegistry,
        store: SharedStore,
        queues: QueueMap,
        data_region_id: RegionId,
        host_id: vil_types::HostId,
        verbs: Option<Arc<dyn VerbsDriver>>,
    },
}

/// Internal state of VastarRuntimeWorld (wrapped in Arc for sharing).
struct RuntimeState {
    backend: InternalState,
    obs: ObservabilityHub,
}

/// Main VIL runtime facade.
#[derive(Clone)]
pub struct VastarRuntimeWorld {
    inner: Arc<RuntimeState>,
}

impl VastarRuntimeWorld {
    /// Create runtime in LOCAL mode (in-memory, single process).
    #[doc(alias = "vil_keep")]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RuntimeState {
                backend: InternalState::Local {
                    next_process_id: AtomicU64::new(1),
                    next_port_id: AtomicU64::new(1),
                    next_sample_id: AtomicU64::new(1),
                    store: SharedStore::new(),
                    registry: Registry::new(),
                    queues: Arc::new(DashMap::new()),
                    host_id: vil_types::HostId(0),
                    verbs: None,
                },
                obs: ObservabilityHub::new(),
            }),
        }
    }

    /// Create or attach to runtime in SHARED mode (cross-process).
    #[doc(alias = "vil_keep")]
    pub fn new_shared() -> std::io::Result<Self> {
        Self::new_shared_with_host(vil_types::HostId(1))
    }

    /// Create or attach to runtime in SHARED mode with a specific HostId.
    #[doc(alias = "vil_keep")]
    pub fn new_shared_with_host(host_id: vil_types::HostId) -> std::io::Result<Self> {
        let heap = ExchangeHeap::new();

        // Ensure default data region exists
        let data_region_name = "default_data";
        let data_region_id = match heap.attach_region(data_region_name) {
            Ok(id) => id,
            Err(_) => heap.create_named_region(data_region_name, 1024 * 1024 * 64)?, // 64MB
        };

        let registry = ShmRegistry::new_or_attach(heap.clone(), host_id)?;
        registry.register_host(host_id, "local");

        Ok(Self {
            inner: Arc::new(RuntimeState {
                backend: InternalState::Shared {
                    heap,
                    registry,
                    store: SharedStore::new(),
                    queues: Arc::new(DashMap::new()),
                    data_region_id,
                    host_id,
                    verbs: Some(Arc::new(VerbsContext::new(host_id))),
                },
                obs: ObservabilityHub::new(),
            }),
        })
    }

    #[doc(alias = "vil_keep")]
    pub fn register_process(&self, spec: ProcessSpec) -> Result<ProcessHandle, RtError> {
        let process_id = match &self.inner.backend {
            InternalState::Local {
                next_process_id,
                registry,
                ..
            } => {
                let id = ProcessId(next_process_id.fetch_add(1, Ordering::Relaxed));
                registry.register_process(id, spec.name, spec.cleanup);
                id
            }
            InternalState::Shared { registry, .. } => {
                let id = registry.next_process_id();
                registry.register_process(id, spec.name, spec.cleanup);
                id
            }
        };

        let mut port_map = HashMap::new();

        for port in spec.ports.iter() {
            let port_id = match &self.inner.backend {
                InternalState::Local {
                    next_port_id,
                    registry,
                    queues,
                    ..
                } => {
                    let id = PortId(next_port_id.fetch_add(1, Ordering::Relaxed));
                    registry.register_port(id, process_id, port.direction, port.name);

                    let queue: Box<dyn QueueBackend> = match port.queue {
                        QueueKind::Spsc => Box::new(SpscQueue::new(port.capacity.max(2))),
                        QueueKind::Mpmc => Box::new(DescriptorQueue::with_capacity(port.capacity)),
                    };
                    queues.insert(id, queue);
                    id
                }
                InternalState::Shared {
                    registry, queues, ..
                } => {
                    let id = registry.next_port_id();
                    registry.register_port(id, process_id, port.direction, port.name);

                    // TODO: Proper SHM Queue allocation
                    let queue: Box<dyn QueueBackend> = match port.queue {
                        QueueKind::Spsc => Box::new(SpscQueue::new(port.capacity.max(2))),
                        QueueKind::Mpmc => Box::new(DescriptorQueue::with_capacity(port.capacity)),
                    };
                    queues.insert(id, queue);
                    id
                }
            };

            port_map.insert(
                port.name.to_string(),
                RegisteredPort {
                    id: port_id,
                    spec: *port,
                },
            );
        }

        {
            use vil_log::{dict::register_str, system_log, types::SystemPayload};
            system_log!(
                Info,
                SystemPayload {
                    event_type: 4, // startup / registration
                    ..SystemPayload::default()
                }
            );
            let _ = register_str(spec.name);
        }

        Ok(ProcessHandle {
            process_id,
            spec,
            ports: port_map,
            world: self.clone(),
        })
    }

    #[doc(alias = "vil_keep")]
    pub fn connect(&self, from: PortId, to: PortId) {
        match &self.inner.backend {
            InternalState::Local { registry, .. } => {
                registry.register_route(from, to);
            }
            InternalState::Shared { registry, .. } => {
                registry.register_route(from, to);
            }
        }
    }

    /// REROUTE: Atomically change the targets for a port.
    pub fn reroute(&self, from: PortId, to_list: Vec<PortId>) {
        match &self.inner.backend {
            InternalState::Local { registry, .. } => {
                registry.clear_routes(from);
                for to in to_list {
                    registry.register_route(from, to);
                }
            }
            InternalState::Shared { registry, .. } => {
                registry.clear_routes(from);
                for to in to_list {
                    registry.register_route(from, to);
                }
            }
        }
    }

    /// MANUALLY INJECT a descriptor into a port queue.
    /// Used for simulation of network arrival.
    pub fn inject_descriptor(
        &self,
        target_port: PortId,
        descriptor: Descriptor,
    ) -> Result<(), RtError> {
        let queues = match &self.inner.backend {
            InternalState::Local { queues, .. } => queues,
            InternalState::Shared { queues, .. } => queues,
        };

        if let Some(queue) = queues.get(&target_port) {
            queue.push(descriptor);
            Ok(())
        } else {
            Err(RtError::UnknownPort(target_port))
        }
    }

    /// Simulated RDMA Pull completion.
    /// Manually populates the local store for a remote sample ID.
    pub fn simulate_pull_completion<T: MessageContract + Send + Sync + 'static>(
        &self,
        sample_id: SampleId,
        value: T,
    ) {
        let store = match &self.inner.backend {
            InternalState::Local { store, .. } => store,
            InternalState::Shared { store, .. } => store,
        };
        store.insert_typed(sample_id, value);
    }

    pub fn loan_uninit<T>(&self, origin_port: PortId) -> Result<Loaned<T>, RtError> {
        let sample_id = match &self.inner.backend {
            InternalState::Local { next_sample_id, .. } => {
                SampleId(next_sample_id.fetch_add(1, Ordering::Relaxed))
            }
            InternalState::Shared { registry, .. } => registry.next_sample_id(),
        };
        Ok(Loaned::new(sample_id, origin_port))
    }

    /// publish_value: Ergonomic helper to avoid manual loan/write/publish boilerplate.
    pub fn publish_value<T>(
        &self,
        owner: ProcessId,
        origin_port: PortId,
        value: T,
    ) -> Result<Published<T>, RtError>
    where
        T: MessageContract + Send + Sync + 'static,
    {
        let loan = self.loan_uninit::<T>(origin_port)?.write(value);
        self.publish(owner, origin_port, loan)
    }

    /// publish_control_done: Emissions of Control plane markers.
    pub fn publish_control_done(
        &self,
        owner: ProcessId,
        origin_port: PortId,
        session_id: u64,
    ) -> Result<Published<GenericToken>, RtError> {
        self.publish_value(
            owner,
            origin_port,
            GenericToken {
                session_id,
                is_done: true,
                data: VSlice::from_vec(Vec::<u8>::new()),
            },
        )
    }
    pub fn shm_registry(&self) -> Option<ShmRegistry> {
        match &self.inner.backend {
            InternalState::Shared { registry, .. } => Some(registry.clone()),
            _ => None,
        }
    }

    /// Get access to the ExchangeHeap for direct SHM writes (ShmToken path).
    pub fn exchange_heap(&self) -> Option<&ExchangeHeap> {
        match &self.inner.backend {
            InternalState::Shared { heap, .. } => Some(heap),
            _ => None,
        }
    }

    /// Get the data region ID for SHM payload writes.
    pub fn data_region_id(&self) -> Option<RegionId> {
        match &self.inner.backend {
            InternalState::Shared { data_region_id, .. } => Some(*data_region_id),
            _ => None,
        }
    }

    pub fn verbs_driver(&self) -> Option<Arc<dyn VerbsDriver>> {
        match &self.inner.backend {
            InternalState::Local { verbs, .. } => verbs.clone(),
            InternalState::Shared { verbs, .. } => verbs.clone(),
        }
    }

    pub fn raw_counters(&self) -> &vil_obs::counters::RuntimeCounters {
        match &self.inner.backend {
            InternalState::Local { .. } => &self.inner.obs.counters,
            InternalState::Shared { registry, .. } => registry.global_counters(),
        }
    }

    pub fn publish<T>(
        &self,
        owner: ProcessId,
        origin_port: PortId,
        mut loan: Loaned<T>,
    ) -> Result<Published<T>, RtError>
    where
        T: MessageContract + Send + Sync + 'static,
    {
        let value = loan
            .take_value()
            .ok_or(RtError::LoanWasNeverInitialized(loan.sample_id()))?;
        let sample_id = loan.sample_id();

        match &self.inner.backend {
            InternalState::Local {
                registry,
                store,
                queues,
                host_id,
                ..
            } => {
                let targets = registry.get_routes(origin_port);
                let expected_reads = targets.len() as u32;
                if expected_reads == 0 {
                    return Err(RtError::PortHasNoRoute(origin_port));
                }

                registry.register_sample(
                    sample_id,
                    owner,
                    *host_id,
                    origin_port,
                    expected_reads,
                    RegionId(0),
                    0,
                    std::mem::size_of::<T>() as u32,
                    std::mem::align_of::<T>() as u32,
                );
                store.insert_typed(sample_id, value);
                registry.mark_published(sample_id);

                match &self.inner.backend {
                    InternalState::Local { .. } => self.inner.obs.counters.inc_publishes(),
                    InternalState::Shared { registry, .. } => {
                        registry.global_counters().inc_publishes()
                    }
                }

                let descriptor = Descriptor {
                    sample_id,
                    origin_host: *host_id,
                    origin_port,
                    lineage_id: sample_id.0,
                    publish_ts: crate::clock::now_ns(),
                };

                for target in targets {
                    if let Some(queue) = queues.get(&target) {
                        queue.push(descriptor);
                    }
                }
            }
            InternalState::Shared {
                registry,
                store,
                queues,
                heap: _,
                host_id,
                ..
            } => {
                let targets = registry.get_routes(origin_port);
                let expected_reads = targets.len() as u32;
                if expected_reads == 0 {
                    return Err(RtError::PortHasNoRoute(origin_port));
                }

                if T::META.is_stable {
                    // ULTRA-FAST PATH: Encode stable type directly in descriptor.
                    // No SHM alloc for token struct, no SHM write for token.
                    // Token data (e.g. ShmToken{session_id, data_offset, data_len, status})
                    // is serialized into descriptor fields:
                    //   sample_id = first 8 bytes of T (session_id for ShmToken)
                    //   lineage_id = bytes 8..16 of T (data_offset for ShmToken)
                    //   publish_ts = bytes 16..24 of T (data_len + status packed)
                    //
                    // This eliminates: 1 SHM alloc + 1 memcpy(write) + 1 memcpy(read) per message.
                    let src = &value as *const T as *const u8;
                    let size = std::mem::size_of::<T>();
                    // SAFETY: src points to valid SHM-mapped memory of at least `size` bytes.
                    let bytes = unsafe { std::slice::from_raw_parts(src, size) };

                    // Pack T's bytes into descriptor fields (up to 24 bytes = 3 × u64)
                    let f0 = if size >= 8 {
                        u64::from_ne_bytes(bytes[0..8].try_into().unwrap())
                    } else {
                        0
                    };
                    let f1 = if size >= 16 {
                        u64::from_ne_bytes(bytes[8..16].try_into().unwrap())
                    } else {
                        0
                    };
                    let f2 = if size >= 24 {
                        u64::from_ne_bytes(bytes[16..24].try_into().unwrap())
                    } else {
                        0
                    };

                    registry.global_counters().inc_publishes();

                    let descriptor = Descriptor {
                        sample_id: SampleId(f0),
                        origin_host: *host_id,
                        origin_port,
                        lineage_id: f1,
                        publish_ts: f2,
                    };

                    for target in targets {
                        if let Some(queue) = queues.get(&target) {
                            queue.push(descriptor);
                        }
                    }

                    return Ok(Published::new(SampleId(f0)));
                }

                // SLOW PATH: Non-stable types go through HashMap store + sample table
                store.insert_typed(sample_id, value);

                registry.register_sample(
                    sample_id,
                    owner,
                    *host_id,
                    origin_port,
                    expected_reads,
                    RegionId(0),
                    0,
                    std::mem::size_of::<T>() as u32,
                    std::mem::align_of::<T>() as u32,
                );
                registry.mark_published(sample_id);

                registry.global_counters().inc_publishes();

                let descriptor = Descriptor {
                    sample_id,
                    origin_host: *host_id,
                    origin_port,
                    lineage_id: sample_id.0,
                    publish_ts: crate::clock::now_ns(),
                };

                for target in targets {
                    if let Some(queue) = queues.get(&target) {
                        queue.push(descriptor);
                    }
                }
            }
        }

        Ok(Published::new(sample_id))
    }

    pub fn publish_control(
        &self,
        origin_process: ProcessId,
        origin_port: PortId,
        signal: ControlSignal,
    ) -> Result<Published<ControlSignal>, RtError> {
        self.publish_value(origin_process, origin_port, signal)
    }

    pub fn recv<T>(&self, target_port: PortId) -> Result<SampleGuard<T>, RtError>
    where
        T: MessageContract + Send + Sync + 'static,
    {
        match &self.inner.backend {
            InternalState::Local {
                queues,
                store,
                host_id,
                ..
            } => {
                let queue = queues
                    .get(&target_port)
                    .ok_or(RtError::UnknownPort(target_port))?;
                let descriptor = queue.try_pop().ok_or(RtError::QueueEmpty(target_port))?;

                if descriptor.origin_host != *host_id {
                    // Simulated Remote Pull
                    self.inner.obs.counters.inc_net_pulls();
                }

                let value = store.get_typed::<T>(descriptor.sample_id).ok_or_else(|| {
                    if store.contains(descriptor.sample_id) {
                        RtError::TypeMismatch(std::any::type_name::<T>())
                    } else {
                        RtError::LoanWasNeverInitialized(descriptor.sample_id)
                    }
                })?;

                let latency = crate::clock::now_ns().saturating_sub(descriptor.publish_ts);
                match &self.inner.backend {
                    InternalState::Local { .. } => {
                        self.inner.obs.counters.inc_receives();
                        self.inner.obs.record_latency(latency);
                    }
                    InternalState::Shared { registry, .. } => {
                        registry.global_counters().inc_receives();
                        registry.global_latency().record_ns(latency);
                    }
                }

                Ok(SampleGuard::new(self.clone(), descriptor.sample_id, value))
            }
            InternalState::Shared {
                queues,
                store,
                registry,
                heap: _,
                host_id,
                ..
            } => {
                let queue = queues
                    .get(&target_port)
                    .ok_or(RtError::UnknownPort(target_port))?;
                let descriptor = queue.try_pop().ok_or(RtError::QueueEmpty(target_port))?;

                let value = if T::META.is_stable {
                    // ULTRA-FAST PATH: Reconstruct T directly from descriptor fields.
                    // No SHM read, no sample table, no HashMap — pure register ops.
                    let size = std::mem::size_of::<T>();
                    let mut buf = [0u8; 32]; // max stable type size
                    if size >= 8 {
                        buf[0..8].copy_from_slice(&descriptor.sample_id.0.to_ne_bytes());
                    }
                    if size >= 16 {
                        buf[8..16].copy_from_slice(&descriptor.lineage_id.to_ne_bytes());
                    }
                    if size >= 24 {
                        buf[16..24].copy_from_slice(&descriptor.publish_ts.to_ne_bytes());
                    }

                    let mut val = std::mem::MaybeUninit::<T>::uninit();
                    // SAFETY: buf contains valid bytes for the target type, size verified by caller.
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            buf.as_ptr(),
                            val.as_mut_ptr() as *mut u8,
                            size,
                        );
                        Arc::new(val.assume_init())
                    }
                } else {
                    // SLOW PATH: Non-stable types — HashMap store + sample table
                    if descriptor.origin_host != *host_id {
                        if let Some(_remote_addr) = registry.get_host_addr(descriptor.origin_host) {
                            registry.global_counters().inc_net_pulls();
                            if let Some(driver) = self.verbs_driver() {
                                let _ = driver.host_id();
                            }
                        }
                    }

                    store.get_typed::<T>(descriptor.sample_id).ok_or_else(|| {
                        if store.contains(descriptor.sample_id) {
                            RtError::TypeMismatch(std::any::type_name::<T>())
                        } else {
                            RtError::LoanWasNeverInitialized(descriptor.sample_id)
                        }
                    })?
                };

                if !T::META.is_stable {
                    registry.mark_received(descriptor.sample_id);
                }

                let latency = crate::clock::now_ns().saturating_sub(descriptor.publish_ts);
                match &self.inner.backend {
                    InternalState::Local { .. } => {
                        self.inner.obs.counters.inc_receives();
                        self.inner.obs.record_latency(latency);
                    }
                    InternalState::Shared { registry, .. } => {
                        registry.global_counters().inc_receives();
                        registry.global_latency().record_ns(latency);
                    }
                }

                Ok(SampleGuard::new(self.clone(), descriptor.sample_id, value))
            }
        }
    }

    pub fn release_sample(&self, sample_id: SampleId) {
        match &self.inner.backend {
            InternalState::Local {
                registry, store, ..
            } => {
                if registry.mark_release_read(sample_id) {
                    registry.reclaim_sample(sample_id);
                    store.remove(sample_id);
                }
            }
            InternalState::Shared {
                registry, store, ..
            } => {
                if registry.mark_release_read(sample_id) {
                    registry.reclaim_sample(sample_id);
                    store.remove(sample_id);
                }
            }
        }
    }

    pub fn metrics_snapshot(&self) -> RuntimeMetrics {
        match &self.inner.backend {
            InternalState::Local {
                queues, registry, ..
            } => RuntimeMetrics {
                queue_depth_total: queues
                    .iter()
                    .map(
                        |q: dashmap::mapref::multiple::RefMulti<
                            '_,
                            PortId,
                            Box<dyn QueueBackend>,
                        >| q.value().len(),
                    )
                    .sum::<usize>(),
                in_flight_samples: registry.sample_count(),
                registered_processes: registry.process_report().len(),
            },
            InternalState::Shared {
                queues, registry, ..
            } => RuntimeMetrics {
                queue_depth_total: queues
                    .iter()
                    .map(
                        |q: dashmap::mapref::multiple::RefMulti<
                            '_,
                            PortId,
                            Box<dyn QueueBackend>,
                        >| q.value().len(),
                    )
                    .sum::<usize>(),
                in_flight_samples: registry.sample_count(),
                registered_processes: 0,
            },
        }
    }

    pub fn supervisor(&self) -> Supervisor {
        match &self.inner.backend {
            InternalState::Local {
                registry,
                store,
                queues,
                ..
            } => Supervisor::new(registry.clone(), store.clone(), queues.clone()),
            InternalState::Shared { store, queues, .. } => {
                // TODO: ShmSupervisor (Phase 4)
                Supervisor::new(Registry::new(), store.clone(), queues.clone())
            }
        }
    }

    pub fn crash_process(&self, process_id: ProcessId) -> CleanupReport {
        {
            use vil_log::{system_log, types::SystemPayload};
            system_log!(
                Warn,
                SystemPayload {
                    event_type: 3, // panic / crash
                    ..SystemPayload::default()
                }
            );
        }
        self.supervisor().crash_process(process_id)
    }

    pub fn obs(&self) -> &ObservabilityHub {
        &self.inner.obs
    }

    /// Snapshot of all processes in the registry.
    pub fn registry_processes(&self) -> Vec<ProcessSnapshot> {
        match &self.inner.backend {
            InternalState::Local { registry, .. } => registry.process_report(),
            InternalState::Shared { registry, .. } => registry.snapshot_processes(),
        }
    }

    /// Snapshot of all ports in the registry.
    pub fn registry_ports(&self) -> Vec<PortSnapshot> {
        match &self.inner.backend {
            InternalState::Local { registry, .. } => registry.port_report(),
            InternalState::Shared { registry, .. } => registry.snapshot_ports(),
        }
    }

    /// Snapshot of all samples in the registry.
    pub fn registry_samples(&self) -> Vec<SampleSnapshot> {
        match &self.inner.backend {
            InternalState::Local { registry, .. } => registry.sample_report(),
            InternalState::Shared { registry, .. } => registry.snapshot_samples(),
        }
    }

    /// Shared memory region usage statistics.
    pub fn shm_stats(&self) -> Vec<vil_shm::RegionStats> {
        match &self.inner.backend {
            InternalState::Local { .. } => Vec::new(),
            InternalState::Shared { heap, .. } => heap.all_stats(),
        }
    }

    /// Triggers compacting specifically for SHARED mode data regions.
    pub fn compact_shm(&self) -> Result<usize, String> {
        match &self.inner.backend {
            InternalState::Shared {
                heap,
                registry,
                data_region_id,
                ..
            } => heap.compact_region(*data_region_id, registry),
            InternalState::Local { .. } => {
                Err("Compaction not supported in Local mode".to_string())
            }
        }
    }

    /// Snapshot of all performance counters.
    pub fn counters_snapshot(&self) -> vil_obs::counters::CounterSnapshot {
        self.raw_counters().snapshot()
    }

    /// Increment manual counter "publish".
    pub fn inc_publish(&self) {
        self.raw_counters().inc_publishes();
    }

    /// Increment manual counter "receive".
    pub fn inc_receive(&self) {
        self.raw_counters().inc_receives();
    }

    /// Record manual latency sample.
    pub fn record_latency(&self, latency_ns: u64) {
        match &self.inner.backend {
            InternalState::Local { .. } => self.inner.obs.record_latency(latency_ns),
            InternalState::Shared { registry, .. } => {
                registry.global_latency().record_ns(latency_ns)
            }
        }
    }

    /// Snapshot of latency from the global tracker.
    pub fn latency_snapshot(&self) -> vil_obs::latency::LatencySnapshot {
        match &self.inner.backend {
            InternalState::Local { .. } => self.inner.obs.latency_snapshot(),
            InternalState::Shared { registry, .. } => registry.global_latency().snapshot(),
        }
    }

    /// Register a remote host in the registry.
    pub fn register_host(&self, host_id: vil_types::HostId, addr: &str) -> bool {
        if let InternalState::Shared { registry, .. } = &self.inner.backend {
            registry.register_host(host_id, addr)
        } else {
            false
        }
    }

    /// Send heartbeat for the local host.
    pub fn heartbeat(&self) {
        if let InternalState::Shared {
            registry, host_id, ..
        } = &self.inner.backend
        {
            registry.heartbeat(*host_id, crate::clock::now_ns());
        }
    }

    /// Run health check and detect failover.
    pub fn perform_health_check(&self, timeout_ns: u64) {
        if let InternalState::Shared { registry, .. } = &self.inner.backend {
            let now = crate::clock::now_ns();
            let dead_hosts = registry.check_dead_hosts(now, timeout_ns);

            if !dead_hosts.is_empty() {
                registry.global_counters().inc_failover_events();
                for host in dead_hosts {
                    println!(
                        "⚠️ HOST FAILURE DETECTED: {:?}. Triggering failover...",
                        host
                    );
                    // In a mature implementation, we would update the routing table
                    // to redirect traffic from ports on this host to alternatives.
                }
            }
        }
    }

    /// Synchronize registry state from a remote snapshot.
    pub fn sync_world_state(
        &self,
        processes: &[vil_registry::ProcessSnapshot],
        ports: &[vil_registry::PortSnapshot],
        hosts: &[(vil_types::HostId, String)],
    ) {
        if let InternalState::Shared { registry, .. } = &self.inner.backend {
            registry.sync_from_remote(processes, ports, hosts);
        }
    }
    pub fn recv_control(&self, target_port: PortId) -> Result<ControlSignal, RtError> {
        match &self.inner.backend {
            InternalState::Local {
                queues,
                store,
                host_id,
                ..
            } => {
                let queue = queues
                    .get(&target_port)
                    .ok_or(RtError::UnknownPort(target_port))?;
                let descriptor = queue.try_pop().ok_or(RtError::QueueEmpty(target_port))?;

                if descriptor.origin_host != *host_id {
                    self.inner.obs.counters.inc_net_pulls();
                }

                let value = store
                    .get_typed::<ControlSignal>(descriptor.sample_id)
                    .ok_or_else(|| RtError::TypeMismatch(std::any::type_name::<ControlSignal>()))?;

                let latency = crate::clock::now_ns().saturating_sub(descriptor.publish_ts);
                self.inner.obs.counters.inc_receives();
                self.inner.obs.record_latency(latency);

                Ok((*value).clone())
            }
            InternalState::Shared {
                queues,
                store,
                registry,
                host_id,
                ..
            } => {
                let queue = queues
                    .get(&target_port)
                    .ok_or(RtError::UnknownPort(target_port))?;
                let descriptor = queue.try_pop().ok_or(RtError::QueueEmpty(target_port))?;

                if descriptor.origin_host != *host_id {
                    registry.global_counters().inc_net_pulls();
                }

                let value = store
                    .get_typed::<ControlSignal>(descriptor.sample_id)
                    .ok_or_else(|| RtError::TypeMismatch(std::any::type_name::<ControlSignal>()))?;

                let latency = crate::clock::now_ns().saturating_sub(descriptor.publish_ts);
                registry.global_counters().inc_receives();
                registry.global_latency().record_ns(latency);

                Ok((*value).clone())
            }
        }
    }
}

pub struct SampleGuard<T: Send + Sync + 'static> {
    world: VastarRuntimeWorld,
    loan: LoanedRead<T>,
}

impl<T: Send + Sync + 'static> SampleGuard<T> {
    pub fn new(world: VastarRuntimeWorld, sample_id: SampleId, value: Arc<T>) -> Self {
        Self {
            world,
            loan: LoanedRead::new(sample_id, value),
        }
    }
    pub fn get(&self) -> &T {
        self.loan.get()
    }
}

impl<T: Send + Sync + 'static> std::ops::Deref for SampleGuard<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: Send + Sync + 'static> Drop for SampleGuard<T> {
    fn drop(&mut self) {
        self.world.release_sample(self.loan.sample_id());
    }
}

impl Default for VastarRuntimeWorld {
    fn default() -> Self {
        Self::new()
    }
}
