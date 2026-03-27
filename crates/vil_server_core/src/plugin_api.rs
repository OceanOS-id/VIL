// =============================================================================
// VIL Server — Plugin Manager REST API
// =============================================================================

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::state::AppState;

/// Create the plugin management router.
pub fn plugin_router() -> Router<AppState> {
    Router::new()
        .route("/admin/plugins", get(list_plugins))
        .route("/admin/plugins/:name", get(get_plugin))
        .route("/admin/plugins/:name/enable", post(enable_plugin))
        .route("/admin/plugins/:name/disable", post(disable_plugin))
        .route("/admin/plugins/:name/config", get(get_config))
        .route("/admin/plugins/:name/config", put(update_config))
        .route("/admin/plugins/:name/test", post(test_plugin))
        .route("/admin/plugins/:name/health", get(plugin_health))
        .route("/admin/plugins/:name", delete(remove_plugin))
        .route("/admin/plugins-gui", get(plugins_gui))
}

/// List all installed plugins.
async fn list_plugins(State(state): State<AppState>) -> impl IntoResponse {
    let plugins = state.plugin_manager().list_plugins();
    axum::Json(serde_json::json!({
        "plugins": plugins,
        "total": plugins.len(),
    }))
}

/// Get plugin detail (manifest + state + masked config).
async fn get_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mgr = state.plugin_manager();

    let manifest = mgr.get_manifest(&name);
    let plugin_state = mgr.get_state(&name);
    let config = mgr.get_config_masked(&name);

    if manifest.is_none() {
        return (StatusCode::NOT_FOUND, axum::Json(serde_json::json!({
            "error": format!("Plugin '{}' not found", name),
        }))).into_response();
    }

    axum::Json(serde_json::json!({
        "manifest": manifest,
        "state": plugin_state,
        "config": config,
    })).into_response()
}

/// Enable a plugin.
async fn enable_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.plugin_manager().enable(&name) {
        Ok(()) => axum::Json(serde_json::json!({
            "status": "enabled",
            "plugin": name,
        })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({
            "error": e,
        }))).into_response(),
    }
}

/// Disable a plugin.
async fn disable_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.plugin_manager().disable(&name) {
        Ok(()) => axum::Json(serde_json::json!({
            "status": "disabled",
            "plugin": name,
        })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({
            "error": e,
        }))).into_response(),
    }
}

/// Get plugin config (secrets masked).
async fn get_config(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.plugin_manager().get_config_masked(&name) {
        Some(config) => axum::Json(serde_json::json!({
            "plugin": name,
            "config": config,
        })).into_response(),
        None => (StatusCode::NOT_FOUND, axum::Json(serde_json::json!({
            "error": format!("Plugin '{}' not found", name),
        }))).into_response(),
    }
}

/// Update plugin config (hot-reload).
async fn update_config(
    State(state): State<AppState>,
    Path(name): Path<String>,
    axum::Json(new_config): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    match state.plugin_manager().update_config(&name, new_config) {
        Ok(changes) => axum::Json(serde_json::json!({
            "status": "config_updated",
            "plugin": name,
            "changes": changes,
            "reload_required": false,
        })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({
            "error": e,
        }))).into_response(),
    }
}

/// Test plugin connection.
async fn test_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Placeholder: in production, call plugin's health_check method
    let is_enabled = state.plugin_manager().is_enabled(&name);
    axum::Json(serde_json::json!({
        "plugin": name,
        "test": if is_enabled { "ok" } else { "plugin not enabled" },
        "enabled": is_enabled,
    }))
}

/// Plugin health status.
async fn plugin_health(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let plugin_state = state.plugin_manager().get_state(&name);
    axum::Json(serde_json::json!({
        "plugin": name,
        "state": plugin_state,
    }))
}

/// Remove a plugin.
async fn remove_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.plugin_manager().remove(&name) {
        Ok(()) => axum::Json(serde_json::json!({
            "status": "removed",
            "plugin": name,
        })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({
            "error": e,
        }))).into_response(),
    }
}

/// Embedded plugin management GUI.
async fn plugins_gui(State(state): State<AppState>) -> impl IntoResponse {
    let plugins = state.plugin_manager().list_plugins();
    let name = state.name();

    let mut plugin_rows = String::new();
    for p in &plugins {
        let state_badge = match &p.state {
            crate::plugin_manifest::PluginState::Enabled => r#"<span style="color:#2ecc71">● Enabled</span>"#,
            crate::plugin_manifest::PluginState::Disabled => r#"<span style="color:#e74c3c">● Disabled</span>"#,
            crate::plugin_manifest::PluginState::Installed => r#"<span style="color:#f39c12">● Installed</span>"#,
            crate::plugin_manifest::PluginState::Error => r#"<span style="color:#e74c3c">✗ Error</span>"#,
        };
        let tier_badge = match &p.tier {
            crate::plugin_manifest::PluginTier::Official => r#"<span style="background:#336791;color:#fff;padding:2px 6px;border-radius:3px;font-size:11px">Official</span>"#,
            crate::plugin_manifest::PluginTier::Community => r#"<span style="background:#555;color:#fff;padding:2px 6px;border-radius:3px;font-size:11px">Community</span>"#,
        };

        plugin_rows.push_str(&format!(
            r#"<tr>
                <td><strong>{name}</strong> {tier}</td>
                <td>{version}</td>
                <td>{state}</td>
                <td>{desc}</td>
                <td>
                    <button onclick="pluginAction('{name}','enable')" style="background:#2ecc71;color:#000;border:none;padding:4px 8px;border-radius:3px;cursor:pointer;margin:1px">Enable</button>
                    <button onclick="pluginAction('{name}','disable')" style="background:#e74c3c;color:#fff;border:none;padding:4px 8px;border-radius:3px;cursor:pointer;margin:1px">Disable</button>
                    <button onclick="location.href='/admin/plugins/{name}'" style="background:#3498db;color:#fff;border:none;padding:4px 8px;border-radius:3px;cursor:pointer;margin:1px">Config</button>
                </td>
            </tr>"#,
            name = p.name, tier = tier_badge, version = p.version,
            state = state_badge, desc = p.description,
        ));
    }

    if plugins.is_empty() {
        plugin_rows = r#"<tr><td colspan="5" style="text-align:center;color:#888;padding:30px">No plugins installed. Use the API to install plugins.</td></tr>"#.to_string();
    }

    let html = format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>{name} — Plugin Manager</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, sans-serif; max-width: 1000px; margin: 40px auto; padding: 0 20px; background: #1a1a2e; color: #e0e0e0; }}
        h1 {{ color: #00d4ff; }}
        table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
        th {{ background: #16213e; padding: 12px; text-align: left; border-bottom: 2px solid #00d4ff; }}
        td {{ padding: 10px 12px; border-bottom: 1px solid #333; }}
        tr:hover {{ background: #16213e; }}
        .nav {{ margin: 20px 0; }}
        .nav a {{ color: #00d4ff; margin-right: 15px; text-decoration: none; }}
        pre {{ background: #0f3460; padding: 15px; border-radius: 8px; overflow-x: auto; }}
    </style>
</head>
<body>
    <h1>Plugin Manager</h1>
    <div class="nav">
        <a href="/admin/playground">Playground</a>
        <a href="/admin/diagnostics">Diagnostics</a>
        <a href="/admin/plugins-gui">Plugins</a>
        <a href="/health">Health</a>
        <a href="/metrics">Metrics</a>
    </div>

    <table>
        <thead>
            <tr><th>Plugin</th><th>Version</th><th>Status</th><th>Description</th><th>Actions</th></tr>
        </thead>
        <tbody>
            {rows}
        </tbody>
    </table>

    <h2>API</h2>
    <pre>
GET  /admin/plugins              List all plugins
GET  /admin/plugins/:name        Plugin detail
POST /admin/plugins/:name/enable  Enable plugin
POST /admin/plugins/:name/disable Disable plugin
GET  /admin/plugins/:name/config  Get config (secrets masked)
PUT  /admin/plugins/:name/config  Update config (hot-reload)
POST /admin/plugins/:name/test    Test connection
    </pre>

    <script>
    async function pluginAction(name, action) {{
        const resp = await fetch(`/admin/plugins/${{name}}/${{action}}`, {{ method: 'POST' }});
        const data = await resp.json();
        alert(JSON.stringify(data, null, 2));
        location.reload();
    }}
    </script>
</body>
</html>"#, name = name, rows = plugin_rows);

    axum::response::Html(html)
}
