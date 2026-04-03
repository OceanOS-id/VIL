// =============================================================================
// VX ServiceProcess — Each service is a VIL Process with Tri-Lane ports
// =============================================================================
//
// A ServiceProcess bundles a set of HTTP endpoints under a common prefix
// with shared private-heap state. In the Tri-Lane model each ServiceProcess
// is a logical VIL Process that communicates via SHM descriptor queues.

use std::any::Any;
use std::sync::Arc;

use axum::extract::Extension;
use axum::http::Method;
use axum::routing::MethodRouter;
use axum::Router;

use super::endpoint::{EndpointDef, ExecClass};
use crate::plugin_system::semantic::{AiLane, AiSemantic, AiSemanticKind};
use crate::router::Visibility;
use crate::state::AppState;

/// A declared AI semantic type for observability.
///
/// Used by community plugins (Level 2 plugin standard) to register
/// the semantic types they emit, fault on, or manage — enabling
/// compile-time and runtime observability validation.
#[derive(Debug, Clone)]
pub struct SemanticDeclaration {
    pub type_name: String,
    pub kind: AiSemanticKind,
    pub lane: AiLane,
}

/// Type-erased layer applicator for Extension injection.
///
/// Each stored closure takes a `Router<AppState>` and returns a new one
/// with the Extension layer applied. This allows ServiceProcess to inject
/// arbitrary Clone + Send + Sync state via axum's Extension extractor.
type LayerFn = Box<dyn FnOnce(Router<AppState>) -> Router<AppState> + Send>;

/// A VIL service process with Tri-Lane ports.
///
/// # Example
/// ```ignore
/// let users = ServiceProcess::new("users")
///     .prefix("/api/users")
///     .endpoint(Method::GET, "/", get(list_users))
///     .endpoint(Method::GET, "/:id", get(get_user))
///     .endpoint(Method::POST, "/", post(create_user))
///     .state(UserState::new(db_pool));
/// ```
pub struct ServiceProcess {
    /// Service name (unique identifier)
    name: String,
    /// URL prefix for all endpoints in this service
    prefix: String,
    /// Visibility: Public (HTTP-exposed) or Internal (mesh-only)
    visibility: Visibility,
    /// Registered endpoints
    endpoints: Vec<EndpointDef>,
    /// Private heap state for this service
    state: Option<Arc<dyn Any + Send + Sync>>,
    /// Default execution class for endpoints in this service
    default_exec: ExecClass,
    /// Extension layers applied to the built router (Phase 1 bridge)
    extensions: Vec<LayerFn>,
    /// Declared AI semantic types (for observability and plugin validation)
    semantic_declarations: Vec<SemanticDeclaration>,
}

impl ServiceProcess {
    /// Create a new service process with the given name.
    ///
    /// Default prefix is `/api/{name}`, visibility is Public.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let prefix = format!("/api/{}", &name);
        Self {
            name,
            prefix,
            visibility: Visibility::Public,
            endpoints: Vec::new(),
            state: None,
            default_exec: ExecClass::default(),
            extensions: Vec::new(),
            semantic_declarations: Vec::new(),
        }
    }

    /// Set the service visibility.
    pub fn visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Set the URL prefix for this service's endpoints.
    pub fn prefix(mut self, path: impl Into<String>) -> Self {
        self.prefix = path.into();
        self
    }

    /// Register an HTTP endpoint with an Axum MethodRouter handler.
    ///
    /// The path is relative to the service prefix.
    pub fn endpoint(
        mut self,
        method: Method,
        path: impl Into<String>,
        handler: MethodRouter<AppState>,
    ) -> Self {
        let path = path.into();
        let handler_name = format!("{}::{}", self.name, &path);
        let def = EndpointDef::new(method, path, handler_name)
            .handler(handler)
            .exec(self.default_exec);
        self.endpoints.push(def);
        self
    }

    /// Set the private heap state for this service.
    ///
    /// State is stored as `Arc<T>` and can be downcast via `ServiceCtx::state::<T>()`.
    /// Also auto-injects as `Extension<T>` so extractors using `Extension<T>` work
    /// without a separate `.extension()` call.
    pub fn state<T: Send + Sync + 'static>(mut self, state: T) -> Self {
        let shared = Arc::new(state);
        self.state = Some(shared.clone());
        // Auto-inject as Extension<Arc<T>> for backward compatibility —
        // handlers can use either ServiceCtx::state::<T>() or Extension<Arc<T>>.
        self.extensions
            .push(Box::new(move |router: Router<AppState>| {
                router.layer(Extension(shared))
            }));
        self
    }

    /// Override the default execution class for all endpoints in this service.
    pub fn exec(mut self, exec_class: ExecClass) -> Self {
        self.default_exec = exec_class;
        self
    }

    /// Add an axum Extension layer to this service's router.
    ///
    /// Handlers can extract the value via `Extension<T>`. This is the
    /// Phase 1 mechanism for injecting shared state into handlers; in
    /// Phase 2 this will be replaced by `ServiceCtx::state::<T>()`.
    ///
    /// # Example
    /// ```ignore
    /// let svc = ServiceProcess::new("tasks")
    ///     .endpoint(Method::GET, "/", get(list_tasks))
    ///     .extension(store);
    /// ```
    pub fn extension<T: Clone + Send + Sync + 'static>(mut self, value: T) -> Self {
        self.extensions
            .push(Box::new(move |router: Router<AppState>| {
                router.layer(Extension(value))
            }));
        self
    }

    // -------------------------------------------------------------------------
    // Semantic declarations (Level 2 plugin standard)
    // -------------------------------------------------------------------------

    /// Declare that this service emits AI events (for observability).
    /// Used by community plugins to register their semantic types.
    ///
    /// ```ignore
    /// ServiceProcess::new("my-svc")
    ///     .emits::<MyEvent>()
    ///     .emits::<AnotherEvent>()
    /// ```
    pub fn emits<T: AiSemantic + 'static>(mut self) -> Self {
        self.semantic_declarations.push(SemanticDeclaration {
            type_name: T::type_name().to_string(),
            kind: T::semantic_kind(),
            lane: T::lane(),
        });
        self
    }

    /// Declare that this service may produce AI faults.
    pub fn faults<T: AiSemantic + 'static>(mut self) -> Self {
        self.semantic_declarations.push(SemanticDeclaration {
            type_name: T::type_name().to_string(),
            kind: T::semantic_kind(),
            lane: T::lane(),
        });
        self
    }

    /// Declare that this service manages AI state.
    pub fn manages<T: AiSemantic + 'static>(mut self) -> Self {
        self.semantic_declarations.push(SemanticDeclaration {
            type_name: T::type_name().to_string(),
            kind: T::semantic_kind(),
            lane: T::lane(),
        });
        self
    }

    /// Get declared semantic types.
    pub fn semantic_declarations(&self) -> &[SemanticDeclaration] {
        &self.semantic_declarations
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get the service name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the URL prefix.
    pub fn prefix_path(&self) -> &str {
        &self.prefix
    }

    /// Get the visibility level.
    pub fn visibility_level(&self) -> Visibility {
        self.visibility
    }

    /// Get the number of registered endpoints.
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    /// Get a reference to the registered endpoints.
    pub fn endpoints(&self) -> &[EndpointDef] {
        &self.endpoints
    }

    /// Get the private heap state Arc (if set).
    pub fn get_state(&self) -> Option<&Arc<dyn Any + Send + Sync>> {
        self.state.as_ref()
    }

    /// Get the default execution class.
    pub fn default_exec_class(&self) -> ExecClass {
        self.default_exec
    }

    // -------------------------------------------------------------------------
    // Phase 1: Axum bridge
    // -------------------------------------------------------------------------

    /// Build an Axum Router from the registered endpoints.
    ///
    /// This is the Phase 1 bridge: VX endpoints are compiled into a standard
    /// Axum router that can be merged into VilServer. In Phase 2 this will
    /// be replaced by direct SHM descriptor dispatch.
    pub fn build_router(&self) -> Router<AppState> {
        self.build_router_inner()
    }

    /// Internal: build the router, consuming extension layers.
    ///
    /// Since `&self` is immutable we cannot move out of `self.extensions`.
    /// Instead `VilApp::run` calls `build_router_owned` when it has
    /// ownership.  The shared-ref path (`build_router`) skips extensions
    /// (backward-compatible for existing callers that don't use extensions).
    fn build_router_inner(&self) -> Router<AppState> {
        let mut router = Router::new();

        for ep in &self.endpoints {
            if let Some(ref handler) = ep.handler {
                router = router.route(&ep.path, handler.clone());
            }
        }

        router
    }

    /// Build the Axum Router, consuming the ServiceProcess to apply
    /// Extension layers that require ownership.
    pub fn build_router_owned(self) -> Router<AppState> {
        let mut router = self.build_router_inner();

        for apply in self.extensions {
            router = apply(router);
        }

        router
    }
}

impl std::fmt::Debug for ServiceProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceProcess")
            .field("name", &self.name)
            .field("prefix", &self.prefix)
            .field("visibility", &self.visibility)
            .field("endpoints", &self.endpoints.len())
            .field("has_state", &self.state.is_some())
            .field("default_exec", &self.default_exec)
            .field("semantic_declarations", &self.semantic_declarations.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_process_defaults() {
        let svc = ServiceProcess::new("orders");
        assert_eq!(svc.name(), "orders");
        assert_eq!(svc.prefix_path(), "/api/orders");
        assert_eq!(svc.visibility_level(), Visibility::Public);
        assert_eq!(svc.endpoint_count(), 0);
        assert_eq!(svc.default_exec_class(), ExecClass::AsyncTask);
    }

    #[test]
    fn service_process_builder() {
        let svc = ServiceProcess::new("products")
            .prefix("/shop/products")
            .visibility(Visibility::Internal)
            .exec(ExecClass::BlockingTask)
            .state(42u32);

        assert_eq!(svc.prefix_path(), "/shop/products");
        assert_eq!(svc.visibility_level(), Visibility::Internal);
        assert_eq!(svc.default_exec_class(), ExecClass::BlockingTask);
        assert!(svc.get_state().is_some());
    }
}
