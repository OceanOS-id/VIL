use crate::message::*;
use crate::provider::*;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub base_url: String,
    pub temperature: Option<f32>,
}

impl AnthropicConfig {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            max_tokens: 4096,
            base_url: "https://api.anthropic.com".into(),
            temperature: None,
        }
    }

    pub fn from_env() -> Self {
        Self::new(
            std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-4-20250514".into()),
        )
    }

    pub fn max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = n;
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn temperature(mut self, t: f32) -> Self {
        self.temperature = Some(t);
        self
    }
}

pub struct AnthropicProvider {
    config: AnthropicConfig,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(config: AnthropicConfig) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
        let __ai_start = std::time::Instant::now();
        // Anthropic requires system message separate from messages array.
        // Extract system messages and convert remaining to Anthropic format.
        let mut system_text = String::new();
        let mut api_messages = Vec::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    if !system_text.is_empty() {
                        system_text.push('\n');
                    }
                    system_text.push_str(&msg.content);
                }
                Role::User => {
                    api_messages.push(serde_json::json!({
                        "role": "user",
                        "content": msg.content,
                    }));
                }
                Role::Assistant => {
                    api_messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": msg.content,
                    }));
                }
                Role::Tool => {
                    // Anthropic uses tool_result content blocks
                    api_messages.push(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": msg.tool_call_id,
                            "content": msg.content,
                        }],
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": api_messages,
        });

        if !system_text.is_empty() {
            body["system"] = serde_json::json!(system_text);
        }
        if let Some(temp) = self.config.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.config.base_url))
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if resp.status() == 401 {
            return Err(LlmError::AuthenticationFailed);
        }
        if resp.status() == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(|s| s * 1000);
            return Err(LlmError::RateLimited {
                retry_after_ms: retry_after,
            });
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(format!("{}: {}", status, text)));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        // Anthropic response: { content: [{type: "text", text: "..."}], model, usage }
        let content = json["content"]
            .as_array()
            .and_then(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b["type"].as_str() == Some("text"))
                    .map(|b| b["text"].as_str().unwrap_or(""))
                    .collect::<Vec<_>>()
                    .first()
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();

        // Extract tool_use blocks if present
        let tool_calls = json["content"]
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b["type"].as_str() == Some("tool_use"))
                    .filter_map(|b| {
                        Some(ToolCall {
                            id: b["id"].as_str()?.to_string(),
                            name: b["name"].as_str()?.to_string(),
                            arguments: b["input"].clone(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty());

        let usage = json["usage"].as_object().map(|u| Usage {
            prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: (u["input_tokens"].as_u64().unwrap_or(0)
                + u["output_tokens"].as_u64().unwrap_or(0)) as u32,
        });

        {
            use vil_log::{ai_log, types::AiPayload};
            let __elapsed = __ai_start.elapsed();
            let (input_tokens, output_tokens) = usage
                .as_ref()
                .map(|u| (u.prompt_tokens, u.completion_tokens))
                .unwrap_or((0, 0));
            ai_log!(
                Info,
                AiPayload {
                    model_hash: vil_log::dict::register_str(self.model()),
                    provider_hash: vil_log::dict::register_str(self.provider_name()),
                    input_tokens,
                    output_tokens,
                    latency_ns: __elapsed.as_nanos() as u64,
                    cost_micro_usd: 0,
                    provider_status: 200,
                    op_type: 0,
                    streaming: 0,
                    retries: 0,
                    cache_hit: 0,
                    meta_bytes: [0; 158],
                }
            );
        }

        Ok(ChatResponse {
            content,
            model: json["model"]
                .as_str()
                .unwrap_or(&self.config.model)
                .to_string(),
            tool_calls,
            usage,
            finish_reason: json["stop_reason"].as_str().map(|s| s.to_string()),
        })
    }

    fn model(&self) -> &str {
        &self.config.model
    }
    fn provider_name(&self) -> &str {
        "anthropic"
    }
}
