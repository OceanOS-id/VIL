// =============================================================================
// VIL Server Mesh Router — Route messages between services
// =============================================================================

use dashmap::DashMap;
use std::sync::Arc;

use super::channel::{mesh_channel, MeshReceiver, MeshSender};
use super::MeshConfig;

/// The mesh router manages channels between services.
pub struct MeshRouter {
    /// Map of "service_name" -> sender channel
    senders: Arc<DashMap<String, MeshSender>>,
    /// Default channel buffer size
    buffer_size: usize,
}

impl MeshRouter {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            senders: Arc::new(DashMap::new()),
            buffer_size,
        }
    }

    /// Register a service and get its receiver channel.
    pub fn register_service(&self, name: impl Into<String>) -> MeshReceiver {
        let name = name.into();
        let (tx, rx) = mesh_channel(self.buffer_size);
        self.senders.insert(name, tx);
        rx
    }

    /// Get a sender handle for a service.
    pub fn sender_for(&self, service: &str) -> Option<MeshSender> {
        self.senders.get(service).map(|s| s.value().clone())
    }

    /// Apply mesh configuration to set up routes.
    pub fn apply_config(&self, _config: &MeshConfig) {
        // Routes are logical — the actual routing happens when services
        // send messages using sender_for(). The config is used for
        // validation and documentation.
        for route in &_config.routes {
            {
                use vil_log::app_log;
                app_log!(Info, "mesh.route.registered", { from: route.from.as_str(), to: route.to.as_str() });
            }
        }
    }
}

impl Default for MeshRouter {
    fn default() -> Self {
        Self::new(1024)
    }
}
