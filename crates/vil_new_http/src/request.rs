// =============================================================================
// vil_new_http::request — Buffered HTTP Request-Response
// =============================================================================
// Simple async request-response path for connectors/workflows that need
// a single HTTP call → JSON response (not streaming).
//
// Does NOT modify the existing streaming pipeline (source.rs / sink.rs).
// =============================================================================

use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

/// Shared global HTTP client — connection-pooled, reused across calls.
fn global_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .tcp_nodelay(true)
            .pool_max_idle_per_host(100)
            .pool_idle_timeout(Some(Duration::from_secs(90)))
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client")
    })
}

/// Error from buffered HTTP request.
#[derive(Debug)]
pub struct HttpRequestError {
    pub message: String,
}

impl std::fmt::Display for HttpRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HttpRequestError {}

/// Response from buffered HTTP request.
#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Value,
}

/// Buffered HTTP request builder — simple POST/GET → JSON response.
///
/// ```ignore
/// use vil_new_http::request::HttpRequest;
///
/// let resp = HttpRequest::post("http://api.example.com/data")
///     .json(serde_json::json!({"name": "Alice"}))
///     .header("Authorization", "Bearer xxx")
///     .send().await?;
///
/// println!("status={}, body={}", resp.status, resp.body);
/// ```
pub struct HttpRequest {
    method: String,
    url: String,
    body: Option<Value>,
    headers: HashMap<String, String>,
    client: Option<Client>,
}

impl HttpRequest {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method: "GET".into(),
            url: url.into(),
            body: None,
            headers: HashMap::new(),
            client: None,
        }
    }

    pub fn post(url: impl Into<String>) -> Self {
        Self {
            method: "POST".into(),
            url: url.into(),
            body: None,
            headers: HashMap::new(),
            client: None,
        }
    }

    pub fn put(url: impl Into<String>) -> Self {
        Self {
            method: "PUT".into(),
            url: url.into(),
            body: None,
            headers: HashMap::new(),
            client: None,
        }
    }

    pub fn delete(url: impl Into<String>) -> Self {
        Self {
            method: "DELETE".into(),
            url: url.into(),
            body: None,
            headers: HashMap::new(),
            client: None,
        }
    }

    pub fn patch(url: impl Into<String>) -> Self {
        Self {
            method: "PATCH".into(),
            url: url.into(),
            body: None,
            headers: HashMap::new(),
            client: None,
        }
    }

    /// Set JSON request body.
    pub fn json(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Add a header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Use an external client (for connection pooling across requests).
    /// If not set, uses the global shared client.
    pub fn client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Send the request and collect the full response as JSON.
    pub async fn send(self) -> Result<HttpResponse, HttpRequestError> {
        let client = self.client.as_ref().unwrap_or_else(|| global_client());

        let mut req = match self.method.as_str() {
            "GET" => client.get(&self.url),
            "POST" => client.post(&self.url),
            "PUT" => client.put(&self.url),
            "DELETE" => client.delete(&self.url),
            "PATCH" => client.patch(&self.url),
            _ => client.post(&self.url),
        };

        for (k, v) in &self.headers {
            req = req.header(k.as_str(), v.as_str());
        }

        if let Some(body) = &self.body {
            req = req.header("Content-Type", "application/json");
            req = req.body(
                serde_json::to_string(body)
                    .map_err(|e| HttpRequestError { message: format!("json serialize: {}", e) })?,
            );
        }

        let resp = req
            .send()
            .await
            .map_err(|e| HttpRequestError { message: format!("HTTP {}: {}", self.method, e) })?;

        let status = resp.status().as_u16();
        let body: Value = resp.json().await.unwrap_or(Value::Null);

        Ok(HttpResponse { status, body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_pattern() {
        let req = HttpRequest::post("http://example.com/api")
            .json(serde_json::json!({"key": "value"}))
            .header("Authorization", "Bearer token");

        assert_eq!(req.method, "POST");
        assert_eq!(req.url, "http://example.com/api");
        assert!(req.body.is_some());
        assert_eq!(req.headers.get("Authorization").unwrap(), "Bearer token");
    }

    #[test]
    fn test_methods() {
        assert_eq!(HttpRequest::get("http://x").method, "GET");
        assert_eq!(HttpRequest::put("http://x").method, "PUT");
        assert_eq!(HttpRequest::delete("http://x").method, "DELETE");
        assert_eq!(HttpRequest::patch("http://x").method, "PATCH");
    }
}
