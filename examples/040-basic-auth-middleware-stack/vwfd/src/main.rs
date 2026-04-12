// 040 — Auth Middleware Stack
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/040-basic-auth-middleware-stack/vwfd/workflows", 8080)
        .native("login_handler", |input| {
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let username = body.get("username").and_then(|v| v.as_str()).unwrap_or("");
            let password = body.get("password").and_then(|v| v.as_str()).unwrap_or("");
            if username == "admin" && password == "vil-demo" {
                Ok(json!({
                    "token": "vil-jwt-demo-token-2024",
                    "expires_in": 3600,
                    "user": username
                }))
            } else {
                Ok(json!({"error": "invalid credentials"}))
            }
        })
        .native("public_info_handler", |_| {
            Ok(json!({
                "service": "auth-middleware-stack",
                "version": "1.0.0",
                "public": true
            }))
        })
        .native("protected_data_handler", |input| {
            let auth_header = input.get("headers")
                .and_then(|h| h.get("authorization"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if auth_header.starts_with("Bearer ") && auth_header.len() > 7 {
                Ok(json!({
                    "data": "sensitive-payload",
                    "access": "granted",
                    "role": "admin"
                }))
            } else {
                Err("unauthorized".into())
            }
        })
        .run()
        .await;
}
