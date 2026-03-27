// =============================================================================
// VIL Server Test — Integration test harness
// =============================================================================
//
// Provides utilities for testing vil-server applications without
// starting a real HTTP server.

pub mod bench;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use bytes::Bytes;
use http_body_util::BodyExt;
use tower::ServiceExt;

/// Test client for vil-server applications.
/// Sends requests directly to the Axum router without network overhead.
pub struct TestClient {
    app: Router,
}

impl TestClient {
    /// Create a new test client wrapping a router.
    pub fn new(app: Router) -> Self {
        Self { app }
    }

    /// Send a GET request and return (status, body).
    pub async fn get(&self, path: &str) -> TestResponse {
        let req = Request::builder()
            .method("GET")
            .uri(path)
            .body(Body::empty())
            .unwrap();

        self.send(req).await
    }

    /// Send a POST request with JSON body.
    pub async fn post_json(&self, path: &str, body: &str) -> TestResponse {
        let req = Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        self.send(req).await
    }

    /// Send a DELETE request.
    pub async fn delete(&self, path: &str) -> TestResponse {
        let req = Request::builder()
            .method("DELETE")
            .uri(path)
            .body(Body::empty())
            .unwrap();

        self.send(req).await
    }

    /// Send a raw request.
    pub async fn send(&self, req: Request<Body>) -> TestResponse {
        let response = self
            .app
            .clone()
            .oneshot(req)
            .await
            .expect("Failed to send request");

        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("Failed to read body")
            .to_bytes();

        TestResponse { status, body }
    }
}

/// Response from a test request.
pub struct TestResponse {
    pub status: StatusCode,
    pub body: Bytes,
}

impl TestResponse {
    /// Get body as string.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    /// Parse body as JSON.
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> T {
        serde_json::from_slice(&self.body).expect("Failed to parse JSON response")
    }

    /// Assert status code.
    pub fn assert_status(&self, expected: StatusCode) {
        assert_eq!(
            self.status, expected,
            "Expected status {}, got {}. Body: {}",
            expected,
            self.status,
            self.text()
        );
    }

    /// Assert status is 200 OK.
    pub fn assert_ok(&self) {
        self.assert_status(StatusCode::OK);
    }

    /// Assert status is 201 Created.
    pub fn assert_created(&self) {
        self.assert_status(StatusCode::CREATED);
    }

    /// Assert status is 404 Not Found.
    pub fn assert_not_found(&self) {
        self.assert_status(StatusCode::NOT_FOUND);
    }
}
