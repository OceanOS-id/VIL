use vil_server::prelude::*;

use std::sync::Arc;
use crate::pool::TensorPool;
use crate::handlers;
use crate::semantic::{TensorAllocEvent, TensorFault, TensorPoolState};

pub struct TensorShmPlugin {
    buffer_count: usize,
    buffer_capacity: usize,
}

impl TensorShmPlugin {
    pub fn new(buffer_count: usize, buffer_capacity: usize) -> Self {
        Self { buffer_count, buffer_capacity }
    }
}

impl VilPlugin for TensorShmPlugin {
    fn id(&self) -> &str { "vil-tensor-shm" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Zero-copy tensor serving via SHM-mapped buffers" }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Service {
            name: "tensor-shm".into(),
            endpoints: vec![
                EndpointSpec::get("/api/tensor/stats"),
            ],
        }]
    }

    fn register(&self, ctx: &mut PluginContext) {
        let pool = Arc::new(TensorPool::new(self.buffer_count, self.buffer_capacity));
        ctx.provide::<Arc<TensorPool>>("tensor-pool", pool.clone());

        let svc = ServiceProcess::new("tensor-shm")
            .endpoint(Method::GET, "/stats", get(handlers::stats_handler))
            .state(pool)
            .emits::<TensorAllocEvent>()
            .faults::<TensorFault>()
            .manages::<TensorPoolState>();
        ctx.add_service(svc);
    }

    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
}
