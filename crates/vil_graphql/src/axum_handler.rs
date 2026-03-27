// =============================================================================
// VIL GraphQL — Axum Route Handlers
// =============================================================================

use axum::routing::get;
use axum::Router;
use vil_server_core::AppState;

use crate::playground;

/// Create the GraphQL router.
///
/// Registers:
///   GET  /graphql/playground → GraphiQL IDE
///   GET  /graphql/schema     → Schema description (JSON)
pub fn graphql_router() -> Router<AppState> {
    Router::new()
        .route("/graphql/playground", get(playground::graphiql_handler))
        .route("/graphql/schema", get(schema_info))
}

async fn schema_info() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "graphql": true,
        "endpoints": {
            "playground": "/graphql/playground",
            "schema": "/graphql/schema",
        },
        "note": "Full GraphQL execution requires entity registration. See vil_graphql docs."
    }))
}
