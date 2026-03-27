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
