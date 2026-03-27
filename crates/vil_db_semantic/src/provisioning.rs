// Datasource registry — startup-time resolution, zero per-request cost.

use dashmap::DashMap;
use std::sync::Arc;

use crate::capability::DbCapability;
use crate::error::{DbError, DbResult};
use crate::provider_trait::DbProvider;

/// Registered datasource with its provider binding.
pub struct DatasourceBinding {
    pub name: String,
    pub provider: Arc<dyn DbProvider>,
    pub required_capabilities: DbCapability,
}

/// Datasource registry — resolves datasource aliases to providers.
/// Populated once at startup. HashMap lookup per query (~10ns).
pub struct DatasourceRegistry {
    bindings: DashMap<String, Arc<DatasourceBinding>>,
}

impl DatasourceRegistry {
    pub fn new() -> Self {
        Self { bindings: DashMap::new() }
    }

    /// Register a datasource with its provider.
    /// Called once at startup during provisioning.
    pub fn register(
        &self,
        name: &str,
        provider: Arc<dyn DbProvider>,
        required_capabilities: DbCapability,
    ) -> DbResult<()> {
        // Validate capabilities at registration time (startup)
        let actual = provider.capabilities();
        if !actual.contains(required_capabilities) {
            return Err(DbError::CapabilityMissing(format!(
                "Datasource '{}': provider '{}' has {} but requires {}",
                name, provider.name(), actual, required_capabilities
            )));
        }

        tracing::info!(
            datasource = %name,
            provider = %provider.name(),
            capabilities = %actual,
            "datasource registered"
        );

        self.bindings.insert(name.to_string(), Arc::new(DatasourceBinding {
            name: name.to_string(),
            provider,
            required_capabilities,
        }));

        Ok(())
    }

    /// Resolve a datasource to its provider.
    /// This is the per-query lookup: 1 DashMap get (~10ns).
    pub fn resolve(&self, name: &str) -> DbResult<Arc<dyn DbProvider>> {
        self.bindings
            .get(name)
            .map(|b| b.provider.clone())
            .ok_or_else(|| DbError::ConnectionFailed(
                format!("Datasource '{}' not registered", name)
            ))
    }

    /// Get binding details for diagnostics.
    pub fn get_binding(&self, name: &str) -> Option<Arc<DatasourceBinding>> {
        self.bindings.get(name).map(|b| b.clone())
    }

    /// List all registered datasources.
    pub fn list(&self) -> Vec<String> {
        self.bindings.iter().map(|e| e.key().clone()).collect()
    }

    /// Health check all datasources.
    pub async fn health_check_all(&self) -> Vec<(String, bool, String)> {
        let mut results = Vec::new();
        for entry in self.bindings.iter() {
            let name = entry.key().clone();
            match entry.value().provider.health_check().await {
                Ok(()) => results.push((name, true, "healthy".into())),
                Err(e) => results.push((name, false, e.to_string())),
            }
        }
        results
    }

    pub fn count(&self) -> usize {
        self.bindings.len()
    }
}

impl Default for DatasourceRegistry {
    fn default() -> Self { Self::new() }
}
