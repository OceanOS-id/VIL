//! HTTP fetch tool — allows the agent to retrieve web content.

use async_trait::async_trait;
use crate::tool::{Tool, ToolError, ToolResult};

/// Tool that fetches content from a URL.
pub struct HttpFetchTool {
    client: reqwest::Client,
}

impl HttpFetchTool {
    /// Create a new HTTP fetch tool with a default client.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for HttpFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a URL targets a private/internal IP range.
/// Blocks SSRF attacks against cloud metadata, internal services, etc.
fn is_private_url(url: &str) -> bool {
    let host = match url.find("://") {
        Some(idx) => {
            let rest = &url[idx + 3..];
            rest.split('/').next().unwrap_or("")
                .split(':').next().unwrap_or("")
        }
        None => return true, // no scheme — reject
    };

    // Block known private/internal hostnames
    if matches!(host,
        "localhost" | "0.0.0.0" | "[::]" | "[::1]"
        | "metadata.google.internal"
        | "metadata.internal"
    ) {
        return true;
    }

    // Parse as IP and check private ranges
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => {
                v4.is_loopback()           // 127.0.0.0/8
                || v4.is_private()         // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                || v4.is_link_local()      // 169.254.0.0/16 (AWS/GCP metadata)
                || v4.is_broadcast()       // 255.255.255.255
                || v4.is_unspecified()     // 0.0.0.0
                || v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64  // 100.64.0.0/10 (CGN)
            }
            std::net::IpAddr::V6(v6) => {
                v6.is_loopback()           // ::1
                || v6.is_unspecified()     // ::
            }
        };
    }

    // Scheme restriction — only http/https allowed
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return true;
    }

    false
}

#[async_trait]
impl Tool for HttpFetchTool {
    fn name(&self) -> &str {
        "http_fetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to fetch"
                },
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST"],
                    "default": "GET"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let url = params["url"]
            .as_str()
            .ok_or(ToolError::InvalidParameters("missing url".into()))?;

        if is_private_url(url) {
            return Err(ToolError::InvalidParameters(
                "URL targets a private/internal address — blocked for security".into(),
            ));
        }

        let method = params["method"].as_str().unwrap_or("GET");

        let resp = match method {
            "POST" => self.client.post(url).send().await,
            _ => self.client.get(url).send().await,
        }
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let text = resp
            .text()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Truncate to 4000 chars to avoid overwhelming LLM context
        let output = if text.len() > 4000 {
            format!("{}...(truncated)", &text[..4000])
        } else {
            text
        };

        Ok(ToolResult {
            output,
            metadata: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_url_blocking() {
        assert!(is_private_url("http://127.0.0.1/secret"));
        assert!(is_private_url("http://10.0.0.1/internal"));
        assert!(is_private_url("http://192.168.1.1/admin"));
        assert!(is_private_url("http://172.16.0.1/api"));
        assert!(is_private_url("http://169.254.169.254/latest/meta-data/")); // AWS metadata
        assert!(is_private_url("http://localhost/admin"));
        assert!(is_private_url("http://0.0.0.0/"));
        assert!(is_private_url("http://[::1]/"));
        assert!(is_private_url("http://metadata.google.internal/"));
        assert!(is_private_url("ftp://example.com/file")); // non-http scheme

        assert!(!is_private_url("https://example.com/api"));
        assert!(!is_private_url("https://8.8.8.8/dns"));
        assert!(!is_private_url("http://203.0.113.1/public"));
    }
}
