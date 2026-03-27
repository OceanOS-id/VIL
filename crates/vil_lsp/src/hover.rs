use tower_lsp::lsp_types::*;
use crate::parser::{VilUsage, UsageKind};

/// Generate hover documentation for a position.
pub fn hover_for(usages: &[VilUsage], position: Position) -> Option<Hover> {
    // Find the usage at or near this position
    let usage = usages.iter().find(|u| {
        u.line == position.line &&
        position.character >= u.col &&
        position.character <= u.col + u.text.len() as u32 + 10
    })?;

    let docs = match &usage.kind {
        UsageKind::SemanticMacro(name) => semantic_docs(name),
        UsageKind::VilApp => vil_app_docs(),
        UsageKind::ServiceProcess => service_process_docs(),
        UsageKind::ExecClass(variant) => exec_class_docs(variant),
        UsageKind::WasmFaaSConfig => wasm_docs(),
        UsageKind::SidecarConfig => sidecar_docs(),
        UsageKind::VilModel => vil_model_docs(),
        UsageKind::VilError => vil_error_docs(),
        UsageKind::EndpointDef => endpoint_docs(&usage.text),
    };

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: docs,
        }),
        range: Some(Range {
            start: Position { line: usage.line, character: usage.col },
            end: Position { line: usage.line, character: usage.col + usage.text.len() as u32 },
        }),
    })
}

fn semantic_docs(name: &str) -> String {
    match name {
        "vil_state" => "## `#[vil_state]`\n\n\
            State machine data type.\n\n\
            - **Lane**: Data Lane only\n\
            - **Transfer**: LoanWrite / LoanRead\n\
            - **Memory**: PagedExchange\n\
            - **Semantics**: Mutable, versioned, single-writer\n\n\
            ```rust\n#[vil_state]\nstruct AppState {\n    count: u32,\n    active: bool,\n}\n```".into(),
        "vil_event" => "## `#[vil_event]`\n\n\
            Immutable event log entry.\n\n\
            - **Lane**: Data Lane or Control Lane\n\
            - **Transfer**: Copy / LoanWrite\n\
            - **Memory**: PagedExchange\n\
            - **Semantics**: Append-only, immutable after creation\n\n\
            ```rust\n#[vil_event]\nstruct OrderPlaced {\n    order_id: u64,\n    amount: f64,\n}\n```".into(),
        "vil_fault" => "## `#[vil_fault]`\n\n\
            Structured fault for error signaling.\n\n\
            - **Lane**: Control Lane only\n\
            - **Transfer**: Copy\n\
            - **Memory**: ControlHeap\n\
            - **Semantics**: Triggers FaultHandler (signal_error, control_abort, degrade)\n\n\
            ```rust\n#[vil_fault]\nstruct ServiceFault {\n    code: u32,\n    message: String,\n}\n```".into(),
        "vil_decision" => "## `#[vil_decision]`\n\n\
            Routing decision for process orchestration.\n\n\
            - **Lane**: Trigger Lane only\n\
            - **Transfer**: Copy\n\
            - **Memory**: ControlHeap\n\
            - **Semantics**: Controls process routing and branching\n\n\
            ```rust\n#[vil_decision]\nstruct RouteDecision {\n    target: String,\n    weight: f32,\n}\n```".into(),
        _ => format!("VIL semantic macro: `{}`", name),
    }
}

fn vil_app_docs() -> String {
    "## `VilApp`\n\n\
     Process-oriented application container.\n\n\
     - Contains one or more `ServiceProcess` instances\n\
     - Manages Tri-Lane IPC, failover, and mesh topology\n\
     - Serves HTTP via Axum\n\n\
     ```rust\nVilApp::new(\"my-app\")\n    .port(8080)\n    .service(my_service)\n    .run()\n    .await;\n```".into()
}

fn service_process_docs() -> String {
    "## `ServiceProcess`\n\n\
     A VIL Process with typed ports and endpoints.\n\n\
     - Each service has its own set of HTTP endpoints\n\
     - Supports middleware, extensions, and exec class configuration\n\n\
     ```rust\nServiceProcess::new(\"api\")\n    .endpoint(Method::GET, \"/users\", get(list_users))\n    .middleware(cors_layer)\n```".into()
}

fn exec_class_docs(variant: &str) -> String {
    match variant {
        "AsyncTask" => "## `ExecClass::AsyncTask`\n\nDefault async execution on tokio runtime.\nBest for I/O-bound handlers.".into(),
        "BlockingTask" => "## `ExecClass::BlockingTask`\n\nRuns on `spawn_blocking` pool.\nBest for CPU-bound work that shouldn't block async runtime.".into(),
        "DedicatedThread" => "## `ExecClass::DedicatedThread`\n\nPinned OS thread.\nBest for long-running background tasks.".into(),
        "PinnedWorker" => "## `ExecClass::PinnedWorker`\n\nCPU-pinned worker.\nBest for latency-sensitive real-time processing.".into(),
        "WasmFaaS" => "## `ExecClass::WasmFaaS`\n\nWASM sandbox execution.\nRequires `.wasm_module(\"name\")` to specify the WASM module.\n\nModule must be registered in `WasmFaaSRegistry`.".into(),
        "SidecarProcess" => "## `ExecClass::SidecarProcess`\n\nExternal sidecar process via UDS.\nRequires `.sidecar_target(\"name\")` to specify the sidecar.\n\nSidecar must be registered in `SidecarRegistry`.".into(),
        _ => format!("ExecClass variant: `{}`", variant),
    }
}

fn wasm_docs() -> String {
    "## `WasmFaaSConfig`\n\nConfiguration for a WASM FaaS module.\n\n\
     - `pool_size`: Number of pre-warmed instances (default: 4)\n\
     - `timeout_ms`: Execution timeout (default: 5000)\n\
     - `max_memory_pages`: Memory limit in 64KB pages (default: 256 = 16MB)".into()
}

fn sidecar_docs() -> String {
    "## `SidecarConfig`\n\nConfiguration for an external sidecar process.\n\n\
     - `command`: Shell command to spawn the sidecar\n\
     - `pool_size`: Connection pool size (default: 4)\n\
     - `timeout_ms`: Invoke timeout (default: 5000)\n\
     - `retry`: Number of retries on transient failure (default: 3)".into()
}

fn vil_model_docs() -> String {
    "## `VilModel` (derive)\n\n\
     Generates `VilModel` trait implementation for zero-copy SHM serialization.\n\n\
     Requires: `Serialize`, `Deserialize`, `Clone`\n\n\
     ```rust\n#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]\nstruct Task { id: u64, title: String }\n```".into()
}

fn vil_error_docs() -> String {
    "## `VilError` (derive)\n\n\
     Generates error handling with HTTP status code mapping.\n\n\
     ```rust\n#[derive(Debug, VilError)]\nenum MyError {\n    #[vil_error(status = 404)]\n    NotFound { id: u64 },\n}\n```".into()
}

fn endpoint_docs(text: &str) -> String {
    format!("## Endpoint: `{}`\n\nRegistered HTTP endpoint in this ServiceProcess.", text)
}
