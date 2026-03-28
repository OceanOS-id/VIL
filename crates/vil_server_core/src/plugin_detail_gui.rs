// =============================================================================
// VIL Server — Admin GUI: Plugin Detail + Config Editor + Metrics
// =============================================================================

use axum::extract::{Path, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;

use crate::state::AppState;

/// Plugin detail GUI router.
pub fn plugin_detail_router() -> Router<AppState> {
    Router::new().route("/admin/plugins-gui/:name", get(plugin_detail_page))
}

async fn plugin_detail_page(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Html<String> {
    let mgr = state.plugin_manager();
    let manifest = mgr.get_manifest(&name);
    let plugin_state = mgr.get_state(&name);
    let config = mgr.get_config_masked(&name);

    let manifest_json = manifest
        .as_ref()
        .map(|m| serde_json::to_string_pretty(m).unwrap_or_default())
        .unwrap_or_else(|| "Plugin not found".to_string());

    let state_json = plugin_state
        .as_ref()
        .map(|s| serde_json::to_string_pretty(s).unwrap_or_default())
        .unwrap_or_default();

    let _config_json = config
        .as_ref()
        .map(|c| serde_json::to_string_pretty(c).unwrap_or_default())
        .unwrap_or_else(|| "{}".to_string());

    // Build config editor form from schema
    let mut form_fields = String::new();
    if let Some(m) = &manifest {
        for (key, field) in &m.config_schema {
            let value = config
                .as_ref()
                .and_then(|c| c.get(key))
                .map(|v| v.to_string().trim_matches('"').to_string())
                .unwrap_or_default();

            let input = match field.field_type.as_str() {
                "enum" => {
                    let options: String = field
                        .values
                        .iter()
                        .map(|v| {
                            format!(
                                r#"<option value="{v}" {sel}>{v}</option>"#,
                                v = v,
                                sel = if *v == value { "selected" } else { "" }
                            )
                        })
                        .collect();
                    format!(
                        r#"<select name="{key}" style="background:#0f3460;color:#e0e0e0;border:1px solid #333;padding:6px;border-radius:4px;width:100%">{options}</select>"#
                    )
                }
                "integer" => {
                    let min = field
                        .min
                        .map(|v| format!(r#" min="{}""#, v))
                        .unwrap_or_default();
                    let max = field
                        .max
                        .map(|v| format!(r#" max="{}""#, v))
                        .unwrap_or_default();
                    format!(
                        r#"<input type="number" name="{key}" value="{value}"{min}{max} style="background:#0f3460;color:#e0e0e0;border:1px solid #333;padding:6px;border-radius:4px;width:100%">"#
                    )
                }
                _ => {
                    let input_type = if field.secret { "password" } else { "text" };
                    let ph = if field.placeholder.is_empty() {
                        ""
                    } else {
                        &field.placeholder
                    };
                    format!(
                        r#"<input type="{input_type}" name="{key}" value="{value}" placeholder="{ph}" style="background:#0f3460;color:#e0e0e0;border:1px solid #333;padding:6px;border-radius:4px;width:100%">"#
                    )
                }
            };

            let required = if field.required { " *" } else { "" };
            form_fields.push_str(&format!(
                r#"<div style="margin:10px 0">
                    <label style="color:#7b68ee;font-size:13px">{label}{req}</label>
                    <div style="font-size:11px;color:#888;margin:2px 0">{desc}</div>
                    {input}
                </div>"#,
                label = if field.label.is_empty() {
                    key.as_str()
                } else {
                    &field.label
                },
                req = required,
                desc = field.description,
                input = input,
            ));
        }
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>{name} — Plugin Config</title>
    <style>
        body {{ font-family: -apple-system, sans-serif; max-width: 900px; margin: 40px auto; padding: 0 20px; background: #1a1a2e; color: #e0e0e0; }}
        h1 {{ color: #00d4ff; }}
        h2 {{ color: #7b68ee; margin-top: 30px; }}
        .card {{ background: #16213e; border-radius: 8px; padding: 20px; margin: 15px 0; }}
        pre {{ background: #0f3460; padding: 12px; border-radius: 6px; overflow-x: auto; font-size: 12px; }}
        button {{ background: #00d4ff; color: #000; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer; font-weight: bold; margin: 5px; }}
        button.danger {{ background: #e74c3c; color: #fff; }}
        button.success {{ background: #2ecc71; color: #000; }}
        .nav a {{ color: #00d4ff; margin-right: 15px; text-decoration: none; }}
        .badge {{ padding: 3px 8px; border-radius: 3px; font-size: 11px; }}
    </style>
</head>
<body>
    <div class="nav">
        <a href="/admin/plugins-gui">← Back to Plugins</a>
        <a href="/admin/playground">Playground</a>
        <a href="/admin/diagnostics">Diagnostics</a>
    </div>

    <h1>{name}</h1>

    <div class="card">
        <h2>Configuration</h2>
        <form id="configForm">
            {fields}
            <div style="margin-top:15px">
                <button type="button" class="success" onclick="saveConfig()">Save Config</button>
                <button type="button" onclick="testConnection()">Test Connection</button>
            </div>
        </form>
    </div>

    <div class="card">
        <h2>State</h2>
        <pre>{state}</pre>
        <button class="success" onclick="pluginAction('enable')">Enable</button>
        <button class="danger" onclick="pluginAction('disable')">Disable</button>
    </div>

    <div class="card">
        <h2>Manifest</h2>
        <pre>{manifest}</pre>
    </div>

    <pre id="result" style="display:none"></pre>

    <script>
    async function saveConfig() {{
        const form = document.getElementById('configForm');
        const data = {{}};
        new FormData(form).forEach((v, k) => {{ data[k] = isNaN(v) ? v : Number(v); }});
        const resp = await fetch('/admin/plugins/{name}/config', {{
            method: 'PUT', headers: {{'Content-Type': 'application/json'}}, body: JSON.stringify(data)
        }});
        const result = await resp.json();
        document.getElementById('result').style.display = 'block';
        document.getElementById('result').textContent = JSON.stringify(result, null, 2);
    }}
    async function testConnection() {{
        const resp = await fetch('/admin/plugins/{name}/test', {{ method: 'POST' }});
        const result = await resp.json();
        alert(JSON.stringify(result, null, 2));
    }}
    async function pluginAction(action) {{
        const resp = await fetch(`/admin/plugins/{name}/${{action}}`, {{ method: 'POST' }});
        const result = await resp.json();
        alert(JSON.stringify(result, null, 2));
        location.reload();
    }}
    </script>
</body>
</html>"#,
        name = name,
        fields = form_fields,
        state = state_json,
        manifest = manifest_json,
    );

    Html(html)
}
