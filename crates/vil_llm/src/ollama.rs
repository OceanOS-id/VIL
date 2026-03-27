use async_trait::async_trait;
use crate::message::*;
use crate::provider::*;

#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

impl OllamaConfig {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            base_url: "http://localhost:11434".into(),
            model: model.into(),
        }
    }

    pub fn from_env() -> Self {
        Self {
            base_url: std::env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434".into()),
            model: std::env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "llama3".into()),
        }
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

pub struct OllamaProvider {
    config: OllamaConfig,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(config: OllamaConfig) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
        let api_messages: Vec<serde_json::Value> = messages.iter().map(|msg| {
            let role = match msg.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            serde_json::json!({
                "role": role,
                "content": msg.content,
            })
        }).collect();

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": api_messages,
            "stream": false,
        });

        let resp = self.client
            .post(format!("{}/api/chat", self.config.base_url))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(format!("{}: {}", status, text)));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        let content = json["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let model = json["model"]
            .as_str()
            .unwrap_or(&self.config.model)
            .to_string();

        Ok(ChatResponse {
            content,
            model,
            tool_calls: None,
            usage: None,
            finish_reason: Some("stop".to_string()),
        })
    }

    fn model(&self) -> &str { &self.config.model }
    fn provider_name(&self) -> &str { "ollama" }
}
