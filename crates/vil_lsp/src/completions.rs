use tower_lsp::lsp_types::*;

/// Generate completions based on trigger text.
pub fn complete(line_text: &str, position: Position) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // Detect context
    let before_cursor = &line_text[..position.character as usize];

    if before_cursor.ends_with("#[vil_") || before_cursor.ends_with("#[vil") {
        // Semantic macro completions
        items.extend(semantic_macro_completions());
    } else if before_cursor.contains("ExecClass::") {
        items.extend(exec_class_completions());
    } else if before_cursor.contains(".endpoint(") || before_cursor.contains("Method::") {
        items.extend(http_method_completions());
    } else if before_cursor.ends_with("VilApp::") || before_cursor.ends_with("vil_app.") {
        items.extend(vil_app_completions());
    } else if before_cursor.ends_with(".service(") || before_cursor.contains("ServiceProcess::") {
        items.extend(service_process_completions());
    } else if before_cursor.contains("async fn") || before_cursor.contains("fn ") {
        // VIL Way handler parameter completions
        items.extend(handler_param_completions());
    } else if before_cursor.contains("body.") {
        items.extend(shm_slice_method_completions());
    } else if before_cursor.contains("ctx.") {
        items.extend(service_ctx_method_completions());
    }

    items
}

fn semantic_macro_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "vil_state".into(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("State machine data (Data Lane, LoanWrite/LoanRead)".into()),
            insert_text: Some("vil_state]".into()),
            ..Default::default()
        },
        CompletionItem {
            label: "vil_event".into(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Immutable event log (Data/Control Lane, Copy)".into()),
            insert_text: Some("vil_event]".into()),
            ..Default::default()
        },
        CompletionItem {
            label: "vil_fault".into(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Structured fault (Control Lane only, Copy)".into()),
            insert_text: Some("vil_fault]".into()),
            ..Default::default()
        },
        CompletionItem {
            label: "vil_decision".into(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Routing decision (Trigger Lane only)".into()),
            insert_text: Some("vil_decision]".into()),
            ..Default::default()
        },
    ]
}

fn exec_class_completions() -> Vec<CompletionItem> {
    vec![
        completion("AsyncTask", "Default tokio async task"),
        completion("BlockingTask", "spawn_blocking for CPU-bound work"),
        completion("DedicatedThread", "Pinned OS thread"),
        completion("PinnedWorker", "CPU-pinned worker thread"),
        completion("WasmFaaS", "WASM sandbox execution (requires .wasm_module())"),
        completion("SidecarProcess", "External sidecar via UDS (requires .sidecar_target())"),
    ]
}

fn http_method_completions() -> Vec<CompletionItem> {
    ["GET", "POST", "PUT", "DELETE", "PATCH"].iter().map(|m| {
        CompletionItem {
            label: m.to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some(format!("HTTP {} method", m)),
            ..Default::default()
        }
    }).collect()
}

fn vil_app_completions() -> Vec<CompletionItem> {
    vec![
        completion("new(\"name\")", "Create a new VilApp"),
        completion("port(8080)", "Set the server port"),
        completion("service(svc)", "Add a ServiceProcess"),
        completion("mesh(config)", "Configure VxMeshConfig"),
        completion("failover(config)", "Configure VxFailoverConfig"),
        completion("observer(true)", "Enable observer dashboard"),
        completion("run().await", "Start the server"),
    ]
}

fn service_process_completions() -> Vec<CompletionItem> {
    vec![
        completion("new(\"name\")", "Create a new ServiceProcess"),
        completion("endpoint(Method::GET, \"/path\", handler)", "Add an endpoint"),
        completion("middleware(layer)", "Add middleware"),
        completion("extension(data)", "Add shared state"),
        completion("exec_class(ExecClass::AsyncTask)", "Set execution class"),
    ]
}

fn completion(label: &str, detail: &str) -> CompletionItem {
    CompletionItem {
        label: label.into(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(detail.into()),
        ..Default::default()
    }
}

// ── VIL Way completions ─────────────────────────────────────

fn handler_param_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "body: ShmSlice".into(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Zero-copy request body from ExchangeHeap".into()),
            insert_text: Some("body: ShmSlice".into()),
            ..Default::default()
        },
        CompletionItem {
            label: "ctx: ServiceCtx".into(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Tri-Lane context — state access + inter-service messaging".into()),
            insert_text: Some("ctx: ServiceCtx".into()),
            ..Default::default()
        },
        CompletionItem {
            label: "shm: ShmContext".into(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("SHM availability + ExchangeHeap region stats".into()),
            insert_text: Some("shm: ShmContext".into()),
            ..Default::default()
        },
        CompletionItem {
            label: "-> VilResponse<T>".into(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Typed JSON response with SIMD serialization".into()),
            insert_text: Some("-> VilResponse<${1:ResponseType}>".into()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "-> Result<VilResponse<T>, VilError>".into(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Handler result with RFC 7807 error mapping".into()),
            insert_text: Some("-> Result<VilResponse<${1:ResponseType}>, VilError>".into()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
    ]
}

fn shm_slice_method_completions() -> Vec<CompletionItem> {
    vec![
        completion("json::<T>()", "Deserialize body as T (SIMD JSON from SHM)"),
        completion("text()", "Body as UTF-8 string slice"),
        completion("as_bytes()", "Raw byte slice (zero-copy from ExchangeHeap)"),
        completion("len()", "Body length in bytes"),
        completion("is_empty()", "True if body is empty"),
        completion("region_id()", "SHM region ID for mesh forwarding"),
        completion("offset()", "Offset within SHM region"),
    ]
}

fn service_ctx_method_completions() -> Vec<CompletionItem> {
    vec![
        completion("state::<T>()", "Downcast service state to concrete type"),
        completion("service_name()", "Name of the owning service process"),
        completion("send(target, data)", "Send on Data Lane (zero-copy payload)"),
        completion("trigger(target, data)", "Send on Trigger Lane (request init)"),
        completion("control(target, data)", "Send on Control Lane (backpressure/health)"),
        completion("tri_lane()", "Access the underlying TriLaneRouter"),
    ]
}
