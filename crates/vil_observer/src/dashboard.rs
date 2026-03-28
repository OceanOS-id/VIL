use axum::{response::Html, routing::get, Router};

const DASHBOARD_HTML: &str = include_str!("dashboard.html");

async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

pub fn dashboard_routes() -> Router {
    Router::new()
        .route("/_vil/dashboard", get(dashboard))
        .route("/_vil/dashboard/", get(dashboard))
}
