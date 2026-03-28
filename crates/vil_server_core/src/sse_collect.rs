//! Generic SSE Stream Collector with dialect support and built-in async client.
//!
//! # Built-in client (no external reqwest dependency needed)
//!
//! ```rust,ignore
//! use vil_server::prelude::*;
//!
//! // Built-in client — zero setup
//! let content = SseCollect::post_to("http://localhost:4545/v1/chat/completions")
//!     .body(json!({"model": "gpt-4", "messages": [...], "stream": true}))
//!     .collect_text().await?;
//!
//! // With dialect
//! let content = SseCollect::post_to(url)
//!     .dialect(SseDialect::anthropic())
//!     .body(body)
//!     .collect_text().await?;
//!
//! // With external client (connection pooling across requests)
//! let content = SseCollect::post(&client, url)
//!     .body(body)
//!     .collect_text().await?;
//! ```
//!
//! # Pre-built dialects
//!
//! | Dialect | Done Signal | Default json_tap |
//! |---------|-------------|------------------|
//! | `SseDialect::openai()` | `data: [DONE]` | `choices[0].delta.content` |
//! | `SseDialect::anthropic()` | `event: message_stop` | `delta.text` |
//! | `SseDialect::ollama()` | `"done": true` (JSON) | `message.content` |
//! | `SseDialect::cohere()` | `event: message-end` | `text` |
//! | `SseDialect::gemini()` | TCP EOF | `candidates[0].content.parts[0].text` |
//! | `SseDialect::standard()` | TCP EOF | (none) |

use bytes::Bytes;
use futures::StreamExt;
use std::sync::OnceLock;

// =============================================================================
// Built-in global async client pool (lazy-initialized, non-blocking)
// =============================================================================

/// Global shared reqwest::Client with sensible defaults for SSE streaming.
/// Lazy-initialized on first use. Non-blocking, connection-pooled.
fn global_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .tcp_nodelay(true)
            .pool_max_idle_per_host(100)
            .pool_idle_timeout(Some(std::time::Duration::from_secs(90)))
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to build SSE client")
    })
}

// =============================================================================
// SSE Dialect
// =============================================================================

/// SSE dialect configuration for a specific provider or convention.
#[derive(Debug, Clone)]
pub struct SseDialect {
    /// Data-line marker that signals end of stream (e.g., "[DONE]").
    pub done_marker: Option<String>,
    /// Named event type that signals end of stream (e.g., "message_stop").
    pub done_event: Option<String>,
    /// JSON field path + value that signals end of stream (e.g., ("done", true)).
    pub done_json_field: Option<(String, serde_json::Value)>,
    /// Default json_tap path for this dialect.
    pub default_tap: Option<String>,
    /// Human-readable name.
    pub name: &'static str,
}

impl SseDialect {
    /// OpenAI / Mistral / Azure OpenAI compatible.
    pub fn openai() -> Self {
        Self {
            done_marker: Some("[DONE]".into()),
            done_event: None,
            done_json_field: None,
            default_tap: Some("choices[0].delta.content".into()),
            name: "openai",
        }
    }

    /// Anthropic Claude.
    pub fn anthropic() -> Self {
        Self {
            done_marker: None,
            done_event: Some("message_stop".into()),
            done_json_field: None,
            default_tap: Some("delta.text".into()),
            name: "anthropic",
        }
    }

    /// Ollama (NDJSON with done field).
    pub fn ollama() -> Self {
        Self {
            done_marker: None,
            done_event: None,
            done_json_field: Some(("done".into(), serde_json::Value::Bool(true))),
            default_tap: Some("message.content".into()),
            name: "ollama",
        }
    }

    /// Cohere.
    pub fn cohere() -> Self {
        Self {
            done_marker: Some("[DONE]".into()),
            done_event: Some("message-end".into()),
            done_json_field: None,
            default_tap: Some("text".into()),
            name: "cohere",
        }
    }

    /// Google Gemini (TCP EOF only).
    pub fn gemini() -> Self {
        Self {
            done_marker: None,
            done_event: None,
            done_json_field: None,
            default_tap: Some("candidates[0].content.parts[0].text".into()),
            name: "gemini",
        }
    }

    /// Pure W3C SSE spec — EOF terminates.
    pub fn standard() -> Self {
        Self {
            done_marker: None,
            done_event: None,
            done_json_field: None,
            default_tap: None,
            name: "standard",
        }
    }

    /// Custom dialect.
    pub fn custom(name: &'static str) -> Self {
        Self {
            done_marker: None,
            done_event: None,
            done_json_field: None,
            default_tap: None,
            name,
        }
    }
}

// =============================================================================
// SseCollectError
// =============================================================================

#[derive(Debug)]
pub enum SseCollectError {
    Request(String),
    Stream(String),
}

impl std::fmt::Display for SseCollectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(e) => write!(f, "SSE request failed: {}", e),
            Self::Stream(e) => write!(f, "SSE stream error: {}", e),
        }
    }
}

impl std::error::Error for SseCollectError {}

// =============================================================================
// SseCollect — Generic SSE stream collector
// =============================================================================

/// Generic SSE stream collector with built-in async client and dialect support.
///
/// POST defaults to OpenAI dialect. GET defaults to W3C standard (EOF only).
pub struct SseCollect {
    client: ClientRef,
    url: String,
    method: HttpMethod,
    json_body: Option<serde_json::Value>,
    headers: Vec<(String, String)>,
    tap_path: Option<String>,
    done_marker: Option<String>,
    done_event: Option<String>,
    done_json_field: Option<(String, serde_json::Value)>,
    event_filter: Option<String>,
}

enum ClientRef {
    /// Built-in global client (zero setup).
    Global,
    /// External client (user-provided, for connection pool reuse).
    External(*const reqwest::Client),
}

// Safety: reqwest::Client is Send+Sync, and External holds a shared ref.
unsafe impl Send for ClientRef {}
unsafe impl Sync for ClientRef {}

impl SseCollect {
    fn client(&self) -> &reqwest::Client {
        match &self.client {
            ClientRef::Global => global_client(),
            ClientRef::External(ptr) => unsafe { &**ptr },
        }
    }

    // ── Constructors with built-in client ───────────────────────────

    /// POST with built-in client (OpenAI dialect default). Zero setup.
    pub fn post_to(url: impl Into<String>) -> Self {
        let d = SseDialect::openai();
        Self {
            client: ClientRef::Global,
            url: url.into(),
            method: HttpMethod::Post,
            json_body: None,
            headers: Vec::new(),
            tap_path: d.default_tap,
            done_marker: d.done_marker,
            done_event: d.done_event,
            done_json_field: d.done_json_field,
            event_filter: None,
        }
    }

    /// GET with built-in client (W3C standard — EOF terminates).
    pub fn get_from(url: impl Into<String>) -> Self {
        Self {
            client: ClientRef::Global,
            url: url.into(),
            method: HttpMethod::Get,
            json_body: None,
            headers: Vec::new(),
            tap_path: None,
            done_marker: None,
            done_event: None,
            done_json_field: None,
            event_filter: None,
        }
    }

    // ── Constructors with external client ────────────────────────────

    /// POST with external client (for connection pool reuse across handlers).
    pub fn post(client: &reqwest::Client, url: impl Into<String>) -> Self {
        let d = SseDialect::openai();
        Self {
            client: ClientRef::External(client as *const _),
            url: url.into(),
            method: HttpMethod::Post,
            json_body: None,
            headers: Vec::new(),
            tap_path: d.default_tap,
            done_marker: d.done_marker,
            done_event: d.done_event,
            done_json_field: d.done_json_field,
            event_filter: None,
        }
    }

    /// GET with external client.
    pub fn get(client: &reqwest::Client, url: impl Into<String>) -> Self {
        Self {
            client: ClientRef::External(client as *const _),
            url: url.into(),
            method: HttpMethod::Get,
            json_body: None,
            headers: Vec::new(),
            tap_path: None,
            done_marker: None,
            done_event: None,
            done_json_field: None,
            event_filter: None,
        }
    }

    // ── Builder methods ─────────────────────────────────────────────

    /// Apply a pre-built dialect.
    pub fn dialect(mut self, d: SseDialect) -> Self {
        self.done_marker = d.done_marker;
        self.done_event = d.done_event;
        self.done_json_field = d.done_json_field;
        if self.tap_path.is_none() {
            self.tap_path = d.default_tap;
        }
        self
    }

    /// Set the JSON body.
    pub fn body(mut self, json: serde_json::Value) -> Self {
        self.json_body = Some(json);
        self
    }

    /// Add a request header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    // ── Auth convenience methods ────────────────────────────────────

    /// Set Bearer token auth (OpenAI, Cohere, Gemini).
    /// Adds `Authorization: Bearer <token>`.
    pub fn bearer_token(self, token: impl Into<String>) -> Self {
        self.header("Authorization", format!("Bearer {}", token.into()))
    }

    /// Set API key header (Anthropic).
    /// Adds `x-api-key: <key>` + `anthropic-version: 2023-06-01`.
    pub fn anthropic_key(self, key: impl Into<String>) -> Self {
        self.header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
    }

    /// Set API key as query parameter (some Gemini endpoints).
    /// Appends `?key=<key>` to the URL.
    pub fn api_key_param(mut self, key: impl Into<String>) -> Self {
        let sep = if self.url.contains('?') { "&" } else { "?" };
        self.url = format!("{}{}key={}", self.url, sep, key.into());
        self
    }

    /// Set json_tap extraction path (overrides dialect default).
    pub fn json_tap(mut self, path: impl Into<String>) -> Self {
        self.tap_path = Some(path.into());
        self
    }

    /// No json_tap — collect raw data lines.
    pub fn raw(mut self) -> Self {
        self.tap_path = None;
        self
    }

    /// Set data-line done marker.
    pub fn done_marker(mut self, marker: impl Into<String>) -> Self {
        self.done_marker = Some(marker.into());
        self
    }

    /// Set named event type that signals done.
    pub fn done_event(mut self, event: impl Into<String>) -> Self {
        self.done_event = Some(event.into());
        self
    }

    /// Set JSON field + value that signals done.
    pub fn done_json_field(mut self, field: impl Into<String>, value: serde_json::Value) -> Self {
        self.done_json_field = Some((field.into(), value));
        self
    }

    /// Only collect data from events matching this event type.
    pub fn event_filter(mut self, event_type: impl Into<String>) -> Self {
        self.event_filter = Some(event_type.into());
        self
    }

    /// Clear all done conditions — stream ends only on TCP EOF.
    pub fn eof_only(mut self) -> Self {
        self.done_marker = None;
        self.done_event = None;
        self.done_json_field = None;
        self
    }

    // ── Execution ───────────────────────────────────────────────────

    /// Collect the entire SSE stream into a single String.
    pub async fn collect_text(self) -> Result<String, SseCollectError> {
        let client = self.client();
        let upstream_url = self.url.clone();
        let start = std::time::Instant::now();

        crate::upstream_metrics::record_start(&upstream_url);

        let mut req = match self.method {
            HttpMethod::Post => client.post(&self.url),
            HttpMethod::Get => client.get(&self.url),
        };

        for (k, v) in &self.headers {
            req = req.header(k.as_str(), v.as_str());
        }

        if let Some(body) = self.json_body {
            req = req.json(&body);
        }

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                let dur = start.elapsed().as_micros() as u64;
                crate::upstream_metrics::record_end(&upstream_url, dur, 0, true);
                return Err(SseCollectError::Request(e.to_string()));
            }
        };
        let status = resp.status().as_u16();

        let mut content = String::new();
        let mut stream = resp.bytes_stream();
        let mut current_event: Option<String> = None;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| SseCollectError::Stream(e.to_string()))?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                // event: field
                if let Some(evt) = line
                    .strip_prefix("event: ")
                    .or_else(|| line.strip_prefix("event:"))
                {
                    let evt = evt.trim();
                    if let Some(ref de) = self.done_event {
                        if evt == de {
                            let dur = start.elapsed().as_micros() as u64;
                            crate::upstream_metrics::record_end(&upstream_url, dur, status, false);
                            return Ok(content);
                        }
                    }
                    current_event = Some(evt.to_string());
                    continue;
                }

                // data: field
                if let Some(data) = line
                    .strip_prefix("data: ")
                    .or_else(|| line.strip_prefix("data:"))
                {
                    let data = data.trim();

                    if let Some(ref dm) = self.done_marker {
                        if data == dm {
                            let dur = start.elapsed().as_micros() as u64;
                            crate::upstream_metrics::record_end(&upstream_url, dur, status, false);
                            return Ok(content);
                        }
                    }

                    if let Some((ref field, ref expected)) = self.done_json_field {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                            if &json[field.as_str()] == expected {
                                let dur = start.elapsed().as_micros() as u64;
                                crate::upstream_metrics::record_end(
                                    &upstream_url,
                                    dur,
                                    status,
                                    false,
                                );
                                return Ok(content);
                            }
                        }
                    }

                    if let Some(ref filter) = self.event_filter {
                        match &current_event {
                            Some(ce) if ce != filter => continue,
                            None if filter != "message" => continue,
                            _ => {}
                        }
                    }

                    if let Some(ref tap) = self.tap_path {
                        let extracted =
                            apply_json_tap(Bytes::copy_from_slice(data.as_bytes()), tap);
                        if !extracted.is_empty() {
                            content.push_str(&String::from_utf8_lossy(&extracted));
                        }
                    } else if !data.is_empty() {
                        content.push_str(data);
                        content.push('\n');
                    }
                }

                // Empty line = event boundary
                if line.is_empty() {
                    current_event = None;
                }
            }
        }

        let dur = start.elapsed().as_micros() as u64;
        crate::upstream_metrics::record_end(&upstream_url, dur, status, false);
        Ok(content)
    }
}

#[derive(Debug, Clone, Copy)]
enum HttpMethod {
    Post,
    Get,
}

// =============================================================================
// json_tap
// =============================================================================

fn apply_json_tap(data: Bytes, path: &str) -> Bytes {
    // Fast path: OpenAI
    if path == "choices[0].delta.content" {
        if let Some(pos) = find_subsequence(&data, b"\"content\":\"") {
            let start = pos + 11;
            if let Some(end) = data[start..].iter().position(|&b| b == b'\"') {
                let content_end = start + end;
                if !data[start..content_end].contains(&b'\\') {
                    return data.slice(start..content_end);
                }
            }
        }
    }

    if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&data) {
        let mut current = &val;
        for part in path.split('.') {
            if let Some(idx_start) = part.find('[') {
                let key = &part[..idx_start];
                current = &current[key];
                if let Some(idx_end) = part.find(']') {
                    if let Ok(idx) = part[idx_start + 1..idx_end].parse::<usize>() {
                        current = &current[idx];
                    }
                }
            } else {
                current = &current[part];
            }
        }
        match current {
            serde_json::Value::String(s) => Bytes::copy_from_slice(s.as_bytes()),
            serde_json::Value::Null => Bytes::new(),
            _ => Bytes::copy_from_slice(current.to_string().as_bytes()),
        }
    } else {
        Bytes::new()
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_tap_openai() {
        let data = Bytes::from(r#"{"choices":[{"delta":{"content":"hello"}}]}"#);
        assert_eq!(
            &apply_json_tap(data, "choices[0].delta.content")[..],
            b"hello"
        );
    }

    #[test]
    fn test_json_tap_anthropic() {
        let data = Bytes::from(r#"{"delta":{"text":"claude says hi"}}"#);
        assert_eq!(&apply_json_tap(data, "delta.text")[..], b"claude says hi");
    }

    #[test]
    fn test_json_tap_ollama() {
        let data = Bytes::from(r#"{"message":{"content":"ollama"}}"#);
        assert_eq!(&apply_json_tap(data, "message.content")[..], b"ollama");
    }

    #[test]
    fn test_json_tap_gemini() {
        let data = Bytes::from(r#"{"candidates":[{"content":{"parts":[{"text":"gem"}]}}]}"#);
        assert_eq!(
            &apply_json_tap(data, "candidates[0].content.parts[0].text")[..],
            b"gem"
        );
    }

    #[test]
    fn test_json_tap_cohere() {
        let data = Bytes::from(r#"{"text":"cohere output"}"#);
        assert_eq!(&apply_json_tap(data, "text")[..], b"cohere output");
    }

    #[test]
    fn test_json_tap_null() {
        let data = Bytes::from(r#"{"choices":[{"delta":{}}]}"#);
        assert!(apply_json_tap(data, "choices[0].delta.content").is_empty());
    }

    #[test]
    fn test_json_tap_nested() {
        let data = Bytes::from(r#"{"a":{"b":"value"}}"#);
        assert_eq!(&apply_json_tap(data, "a.b")[..], b"value");
    }

    #[test]
    fn test_dialect_openai() {
        let d = SseDialect::openai();
        assert_eq!(d.done_marker.as_deref(), Some("[DONE]"));
        assert_eq!(d.default_tap.as_deref(), Some("choices[0].delta.content"));
    }

    #[test]
    fn test_dialect_anthropic() {
        let d = SseDialect::anthropic();
        assert_eq!(d.done_event.as_deref(), Some("message_stop"));
        assert_eq!(d.default_tap.as_deref(), Some("delta.text"));
    }

    #[test]
    fn test_dialect_ollama() {
        let d = SseDialect::ollama();
        let (f, v) = d.done_json_field.unwrap();
        assert_eq!(f, "done");
        assert_eq!(v, serde_json::Value::Bool(true));
    }

    #[test]
    fn test_global_client_initialized() {
        let c = global_client();
        // Just verify it doesn't panic
        let _ = c;
    }
}
