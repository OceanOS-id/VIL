// =============================================================================
// VIL Server — API Playground & Middleware Introspection
// =============================================================================
//
// GET /admin/playground — Embedded API explorer (HTML page)
// GET /admin/middleware — List all registered middleware layers
// GET /admin/routes    — List all registered routes

use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::state::AppState;

/// Create the playground/introspection router.
pub fn playground_router() -> Router<AppState> {
    Router::new()
        .route("/admin/playground", get(playground_page))
        .route("/admin/middleware", get(middleware_list))
        .route("/admin/routes", get(routes_list))
}

/// Embedded API playground — a simple HTML page for testing endpoints.
async fn playground_page(State(state): State<AppState>) -> impl IntoResponse {
    let name = state.name();
    let html = format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>{name} — API Playground</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, sans-serif; max-width: 800px; margin: 40px auto; padding: 0 20px; background: #1a1a2e; color: #e0e0e0; }}
        h1 {{ color: #00d4ff; }}
        h2 {{ color: #7b68ee; margin-top: 30px; }}
        .endpoint {{ background: #16213e; border-radius: 8px; padding: 15px; margin: 10px 0; border-left: 4px solid #00d4ff; }}
        .method {{ font-weight: bold; padding: 2px 8px; border-radius: 4px; font-size: 12px; }}
        .get {{ background: #2ecc71; color: #000; }}
        .post {{ background: #e67e22; color: #000; }}
        .delete {{ background: #e74c3c; color: #fff; }}
        code {{ background: #0f3460; padding: 2px 6px; border-radius: 3px; }}
        input, textarea {{ background: #0f3460; border: 1px solid #333; color: #e0e0e0; padding: 8px; border-radius: 4px; width: 100%; box-sizing: border-box; }}
        button {{ background: #00d4ff; color: #000; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer; font-weight: bold; }}
        button:hover {{ background: #00b8d9; }}
        pre {{ background: #0f3460; padding: 15px; border-radius: 8px; overflow-x: auto; }}
        .status {{ font-size: 14px; color: #7b68ee; }}
    </style>
</head>
<body>
    <h1>{name} — API Playground</h1>
    <p class="status">Powered by vil-server | <a href="/health" style="color:#00d4ff">Health</a> | <a href="/metrics" style="color:#00d4ff">Metrics</a> | <a href="/admin/diagnostics" style="color:#00d4ff">Diagnostics</a></p>

    <h2>Quick Test</h2>
    <div>
        <select id="method" style="width:100px;background:#0f3460;color:#e0e0e0;border:1px solid #333;padding:8px;border-radius:4px">
            <option>GET</option><option>POST</option><option>PUT</option><option>DELETE</option>
        </select>
        <input id="url" value="/" placeholder="Path" style="width:60%;display:inline-block">
        <button onclick="sendRequest()">Send</button>
    </div>
    <textarea id="body" placeholder="Request body (JSON)" rows="3" style="margin-top:10px"></textarea>
    <pre id="response">Response will appear here...</pre>

    <h2>Built-in Endpoints</h2>
    <div class="endpoint"><span class="method get">GET</span> <code>/health</code> — Liveness probe</div>
    <div class="endpoint"><span class="method get">GET</span> <code>/ready</code> — Readiness probe</div>
    <div class="endpoint"><span class="method get">GET</span> <code>/metrics</code> — Prometheus metrics</div>
    <div class="endpoint"><span class="method get">GET</span> <code>/info</code> — Server info</div>
    <div class="endpoint"><span class="method get">GET</span> <code>/admin/diagnostics</code> — Runtime diagnostics</div>
    <div class="endpoint"><span class="method get">GET</span> <code>/admin/traces</code> — Recent traces</div>
    <div class="endpoint"><span class="method get">GET</span> <code>/admin/errors</code> — Error tracker</div>
    <div class="endpoint"><span class="method get">GET</span> <code>/admin/shm</code> — SHM regions</div>
    <div class="endpoint"><span class="method get">GET</span> <code>/admin/capsules</code> — WASM capsules</div>
    <div class="endpoint"><span class="method get">GET</span> <code>/admin/middleware</code> — Middleware stack</div>
    <div class="endpoint"><span class="method get">GET</span> <code>/admin/routes</code> — Registered routes</div>
    <div class="endpoint"><span class="method post">POST</span> <code>/admin/config/reload</code> — Hot reload config</div>

    <script>
    async function sendRequest() {{
        const method = document.getElementById('method').value;
        const url = document.getElementById('url').value;
        const body = document.getElementById('body').value;
        const pre = document.getElementById('response');
        try {{
            const opts = {{ method, headers: {{'Content-Type': 'application/json'}} }};
            if (body && method !== 'GET') opts.body = body;
            const start = performance.now();
            const resp = await fetch(url, opts);
            const duration = (performance.now() - start).toFixed(1);
            const text = await resp.text();
            let formatted;
            try {{ formatted = JSON.stringify(JSON.parse(text), null, 2); }} catch {{ formatted = text; }}
            pre.textContent = `Status: ${{resp.status}} (${{duration}}ms)\n\n${{formatted}}`;
        }} catch (e) {{ pre.textContent = 'Error: ' + e.message; }}
    }}
    </script>
</body>
</html>"#);

    Html(html)
}

/// List registered middleware layers.
async fn middleware_list(State(_state): State<AppState>) -> impl IntoResponse {
    // Enumerate known middleware
    let middleware = vec![
        middleware_info("request_tracker", "Request ID + timing + error tracking", true),
        middleware_info("handler_metrics", "Per-route Prometheus metrics (zero-annotation)", true),
        middleware_info("tracing_middleware", "Distributed tracing (W3C traceparent)", true),
        middleware_info("cors", "CORS permissive", true),
        middleware_info("trace_layer", "Tower HTTP tracing", true),
        middleware_info("security_headers", "OWASP security headers", false),
        middleware_info("compression", "Response compression (gzip)", false),
        middleware_info("timeout", "Request timeout", false),
    ];

    axum::Json(serde_json::json!({
        "middleware": middleware,
        "total": middleware.len(),
    }))
}

fn middleware_info(name: &str, description: &str, enabled: bool) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "description": description,
        "enabled": enabled,
    })
}

/// List registered routes.
async fn routes_list(State(state): State<AppState>) -> impl IntoResponse {
    let handler_keys = state.process_registry().handler_keys();
    let tracked_routes: Vec<String> = state.handler_metrics()
        .to_prometheus()
        .lines()
        .filter(|l| l.starts_with("vil_handler_requests_total"))
        .map(|l| l.to_string())
        .collect();

    axum::Json(serde_json::json!({
        "handler_processes": handler_keys,
        "tracked_routes": tracked_routes,
    }))
}
