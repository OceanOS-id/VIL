// =============================================================================
// VIL Server Router — Service-aware routing
// =============================================================================

use crate::state::AppState;
use axum::Router;

/// A named service with its own route namespace.
/// In unified mode, multiple services share one binary.
pub struct ServiceDef {
    /// Service name (used for logging, metrics, discovery)
    pub name: String,
    /// Route prefix (e.g., "/api/auth")
    pub prefix: String,
    /// Axum router for this service
    pub router: Router<AppState>,
    /// Visibility: public (exposed on main port) or internal (mesh only)
    pub visibility: Visibility,
}

/// Service visibility determines whether routes are exposed on the public port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Exposed on the public HTTP port
    Public,
    /// Only accessible via internal service mesh
    Internal,
}

impl ServiceDef {
    pub fn new(name: impl Into<String>, router: Router<AppState>) -> Self {
        let name = name.into();
        let prefix = format!("/api/{}", &name);
        Self {
            name,
            prefix,
            router,
            visibility: Visibility::Public,
        }
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    pub fn visibility(mut self, vis: Visibility) -> Self {
        self.visibility = vis;
        self
    }

    pub fn internal(mut self) -> Self {
        self.visibility = Visibility::Internal;
        self
    }
}

/// Merge multiple service definitions into a single Axum router.
pub fn merge_services(services: Vec<ServiceDef>) -> Router<AppState> {
    let mut app = Router::new();

    for svc in services {
        if svc.visibility == Visibility::Public {
            app = app.nest(&svc.prefix, svc.router);
        }
        // Internal services are accessible only via mesh (future implementation)
    }

    app
}
