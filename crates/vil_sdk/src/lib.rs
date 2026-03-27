// =============================================================================

pub use vil_types::*;

// GenericToken and its FromStreamData implementation are now in core crates (types/http).

/// Namespace for Runtime World & Handle
pub mod rt {
    pub use vil_engine::rt::error::RtError;
    pub use vil_engine::rt::handle;
    pub use vil_engine::rt::handle::{ProcessHandle, RegisteredPort};
    pub use vil_engine::rt::world::VastarRuntimeWorld;
}

/// Namespace for Core Types
pub mod types {
    pub use vil_types::*;
}

/// Namespace for HTTP Gateway (Native Nodes)
pub mod http {
    pub use vil_new_http::sink::{HttpSink, HttpSinkBuilder};
    pub use vil_new_http::source::{
        FromStreamData, HttpSource, HttpSourceBuilder, SseSourceDialect, WorkflowBuilderExt,
    };
    pub use vil_new_http::HttpFormat;
}

/// Namespace for Semantic IR & Builder
pub mod ir {
    pub use vil_ir::builder::*;
    pub use vil_ir::core::*;
}

/// Namespace for Validator
pub mod validate {
    pub use vil_validate::Validator;
}

pub use vil_macros::message;
pub use vil_macros::process;
pub use vil_macros::trace_hop;
pub use vil_macros::latency_marker;
pub use vil_macros::vil_obs_trace_hop;
pub use vil_macros::vil_obs_latency_label;
pub use vil_macros::vil_decision;
pub use vil_macros::vil_event;
pub use vil_macros::vil_fault;
/// Semantic Type Macros
pub use vil_macros::vil_state;
/// Proc-Macros for Workflow & Messages
pub use vil_macros::vil_workflow;

/// Extension for HTTP (Native Nodes)
pub mod new_http {
    pub use vil_new_http::*;
}

// =============================================================================
// Layer 1 API — "Just Works" (5 lines)
// =============================================================================

/// Create a zero-copy HTTP gateway with minimal configuration.
///
/// # Example (Layer 1 — 5 lines)
/// ```no_run
/// use vil_sdk::prelude::*;
///
/// fn main() {
///     vil_sdk::http_gateway()
///         .listen(3080)
///         .upstream("http://localhost:18081/api/v1/credits/stream")
///         .run();
/// }
/// ```
pub fn http_gateway() -> GatewayBuilder {
    GatewayBuilder::new()
}

/// Builder for the Layer 1 "Just Works" API.
pub struct GatewayBuilder {
    port: u16,
    path: String,
    upstream_url: String,
    json_tap: String,
    sse: bool,
    post_body: Option<serde_json::Value>,
}

impl GatewayBuilder {
    pub fn new() -> Self {
        Self {
            port: 3080,
            path: "/trigger".to_string(),
            upstream_url: String::new(),
            json_tap: String::new(),
            sse: true,
            post_body: None,
        }
    }

    /// Set the port to listen on (default: 3080).
    pub fn listen(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the webhook path (default: "/trigger").
    pub fn path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }

    /// Set the upstream SSE/HTTP URL (e.g., Core Banking SSE stream).
    pub fn upstream(mut self, url: &str) -> Self {
        self.upstream_url = url.to_string();
        self
    }

    /// Set the JSON path to extract from SSE responses (default: empty — pass full payload).
    pub fn json_tap(mut self, tap: &str) -> Self {
        self.json_tap = tap.to_string();
        self
    }

    /// Set the POST body to send to the upstream (optional).
    pub fn post_json(mut self, body: serde_json::Value) -> Self {
        self.post_body = Some(body);
        self
    }

    /// Disable SSE mode (use plain HTTP proxy instead).
    pub fn plain_http(mut self) -> Self {
        self.sse = false;
        self
    }

    /// Build and run the gateway pipeline (blocking).
    ///
    /// Registers sink and source as processes, wires Tri-Lane routes
    /// (Trigger, Data, Control), then runs both workers.
    pub fn run(self) {
        let world = std::sync::Arc::new(
            rt::VastarRuntimeWorld::new_shared().expect("Failed to initialize VIL SHM runtime"),
        );

        let format = if self.sse {
            http::HttpFormat::SSE
        } else {
            http::HttpFormat::Raw
        };

        let sink_builder = http::HttpSinkBuilder::new("Gateway")
            .port(self.port)
            .path(&self.path)
            .out_port("trigger_out")
            .in_port("data_in")
            .ctrl_in_port("ctrl_in");

        let mut source_builder = http::HttpSourceBuilder::new("Upstream")
            .url(&self.upstream_url)
            .format(format)
            .json_tap(&self.json_tap)
            .in_port("trigger_in")
            .out_port("data_out")
            .ctrl_out_port("ctrl_out");

        if let Some(body) = self.post_body {
            source_builder = source_builder.post_json(body);
        }

        // Register processes with runtime
        let sink_h = world
            .register_process(sink_builder.build_spec())
            .expect("Failed to register sink process");
        let source_h = world
            .register_process(source_builder.build_spec())
            .expect("Failed to register source process");

        // Wire Tri-Lane routes: Trigger, Data, Control
        // This is the critical step — without this, sink and source are
        // registered but NOT connected, so messages never flow.
        let sink_trigger_out = sink_h
            .port_id("trigger_out")
            .expect("sink trigger_out port not found");
        let source_trigger_in = source_h
            .port_id("trigger_in")
            .expect("source trigger_in port not found");
        world.connect(sink_trigger_out, source_trigger_in);

        let source_data_out = source_h
            .port_id("data_out")
            .expect("source data_out port not found");
        let sink_data_in = sink_h
            .port_id("data_in")
            .expect("sink data_in port not found");
        world.connect(source_data_out, sink_data_in);

        let source_ctrl_out = source_h
            .port_id("ctrl_out")
            .expect("source ctrl_out port not found");
        let sink_ctrl_in = sink_h
            .port_id("ctrl_in")
            .expect("sink ctrl_in port not found");
        world.connect(source_ctrl_out, sink_ctrl_in);

        // Build and run workers
        let sink_node = http::HttpSink::from_builder(sink_builder);
        let source_node = http::HttpSource::from_builder(source_builder);

        let t1 = sink_node.run_worker::<GenericToken>(world.clone(), sink_h);
        let t2 = source_node.run_worker::<GenericToken>(world.clone(), source_h);

        t1.join().expect("Gateway sink worker panicked");
        t2.join().expect("Gateway source worker panicked");
    }
}

impl Default for GatewayBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Layer 2 API — "Customizable" (20 lines)
// =============================================================================

/// A higher-level pipeline builder for multi-node configurations.
///
/// # Example (Layer 2 — 20 lines)
/// ```ignore
/// use vil_sdk::prelude::*;
///
/// fn main() {
///     let mut pipeline = vil_sdk::Pipeline::new("credit-ingest");
///
///     let sink = pipeline.http_sink()
///         .port(3080).path("/trigger")
///         .out_port("trigger_out")
///         .in_port("data_in")
///         .ctrl_in_port("ctrl_in");
///
///     let source = pipeline.http_source()
///         .url("http://localhost:18081/api/v1/credits/stream")
///         .format(vil_sdk::http::HttpFormat::SSE)
///         .in_port("trigger_in")
///         .out_port("data_out")
///         .ctrl_out_port("ctrl_out");
///
///     pipeline.route(&sink, "trigger_out", &source, "trigger_in", RouteMode::LoanWrite);
///     pipeline.route(&source, "data_out", &sink, "data_in", RouteMode::LoanWrite);
///     pipeline.route(&source, "ctrl_out", &sink, "ctrl_in", RouteMode::Copy);
///
///     pipeline.run();
/// }
/// ```
pub struct Pipeline {
    name: String,
    routes: Vec<PipelineRoute>,
    sink_builder: Option<http::HttpSinkBuilder>,
    source_builder: Option<http::HttpSourceBuilder>,
}

/// A declared route between two named ports in the pipeline.
#[derive(Debug, Clone)]
struct PipelineRoute {
    from_port_name: String,
    to_port_name: String,
    #[allow(dead_code)]
    mode: RouteMode,
}

/// Route transfer mode for Layer 2 API.
#[derive(Debug, Clone, Copy)]
pub enum RouteMode {
    /// Zero-copy write via SHM (for data payloads).
    LoanWrite,
    /// Direct read from SHM buffer.
    LoanRead,
    /// Copy data (for small control messages).
    Copy,
}

/// Trait for pipeline nodes that expose named ports.
pub trait PipelineNode {
    /// Get the node name.
    fn node_name(&self) -> &str;
}

impl PipelineNode for http::HttpSinkBuilder {
    fn node_name(&self) -> &str {
        &self.name
    }
}

impl PipelineNode for http::HttpSourceBuilder {
    fn node_name(&self) -> &str {
        &self.name
    }
}

impl Pipeline {
    /// Create a new pipeline with the given name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            routes: Vec::new(),
            sink_builder: None,
            source_builder: None,
        }
    }

    /// Create an HTTP sink builder (webhook listener).
    pub fn http_sink(&self) -> http::HttpSinkBuilder {
        http::HttpSinkBuilder::new(&format!("{}_sink", self.name))
    }

    /// Create an HTTP source builder (upstream connector).
    pub fn http_source(&self) -> http::HttpSourceBuilder {
        http::HttpSourceBuilder::new(&format!("{}_source", self.name))
    }

    /// Declare a route between two named ports on any pipeline nodes.
    ///
    /// Routes are stored and wired when `run()` is called. The port names
    /// must match the ports declared on the sink/source builders.
    pub fn route(
        &mut self,
        _from_node: &dyn PipelineNode,
        from_port: &str,
        _to_node: &dyn PipelineNode,
        to_port: &str,
        mode: RouteMode,
    ) {
        self.routes.push(PipelineRoute {
            from_port_name: from_port.to_string(),
            to_port_name: to_port.to_string(),
            mode,
        });
    }

    /// Set the sink builder for this pipeline.
    pub fn set_sink(&mut self, builder: http::HttpSinkBuilder) {
        self.sink_builder = Some(builder);
    }

    /// Set the source builder for this pipeline.
    pub fn set_source(&mut self, builder: http::HttpSourceBuilder) {
        self.source_builder = Some(builder);
    }

    /// Build and run the pipeline (blocking).
    ///
    /// Registers sink and source as processes, wires all declared routes
    /// via `world.connect()`, then runs both workers.
    pub fn run(self) {
        let sink_builder = self
            .sink_builder
            .expect("Pipeline requires a sink (call set_sink)");
        let source_builder = self
            .source_builder
            .expect("Pipeline requires a source (call set_source)");

        let world = std::sync::Arc::new(
            rt::VastarRuntimeWorld::new_shared().expect("Failed to initialize VIL SHM runtime"),
        );

        // Register processes with the runtime
        let sink_h = world
            .register_process(sink_builder.build_spec())
            .expect("Failed to register sink process");
        let source_h = world
            .register_process(source_builder.build_spec())
            .expect("Failed to register source process");

        // Wire all declared routes via world.connect()
        for route in &self.routes {
            // Try to resolve from_port from sink first, then source
            let from_id = sink_h
                .port_id(&route.from_port_name)
                .or_else(|_| source_h.port_id(&route.from_port_name))
                .unwrap_or_else(|_| {
                    panic!(
                        "Port '{}' not found on any registered process",
                        route.from_port_name
                    )
                });

            let to_id = source_h
                .port_id(&route.to_port_name)
                .or_else(|_| sink_h.port_id(&route.to_port_name))
                .unwrap_or_else(|_| {
                    panic!(
                        "Port '{}' not found on any registered process",
                        route.to_port_name
                    )
                });

            world.connect(from_id, to_id);
        }

        // Build and run workers
        let sink_node = http::HttpSink::from_builder(sink_builder);
        let source_node = http::HttpSource::from_builder(source_builder);

        let t1 = sink_node.run_worker::<GenericToken>(world.clone(), sink_h);
        let t2 = source_node.run_worker::<GenericToken>(world.clone(), source_h);

        t1.join().expect("Pipeline sink worker panicked");
        t2.join().expect("Pipeline source worker panicked");
    }

    /// Get the pipeline name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub mod prelude {
    pub use crate::new_http::*;
    pub use crate::rt::*;
    pub use crate::GenericToken;
    pub use crate::{GatewayBuilder, Pipeline, PipelineNode, RouteMode};
    pub use vil_macros::process;
    pub use vil_macros::trace_hop;
    pub use vil_macros::latency_marker;
    pub use vil_macros::vil_obs_trace_hop;
    pub use vil_macros::vil_obs_latency_label;
    pub use vil_macros::vil_decision;
    pub use vil_macros::vil_event;
    pub use vil_macros::vil_fault;
    pub use vil_macros::vil_state;
    pub use vil_macros::vil_workflow;
    pub use vil_types::ShmToken;
    pub use vil_types::*;
}
