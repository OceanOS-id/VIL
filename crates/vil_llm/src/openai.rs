use crate::message::*;
use crate::provider::*;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

impl OpenAiConfig {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            base_url: "https://api.openai.com/v1".into(),
            max_tokens: None,
            temperature: None,
        }
    }

    pub fn from_env() -> Self {
        Self::new(
            std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".into()),
        )
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = Some(n);
        self
    }

    pub fn temperature(mut self, t: f32) -> Self {
        self.temperature = Some(t);
        self
    }
}

pub struct OpenAiProvider {
    config: OpenAiConfig,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(config: OpenAiConfig) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(&self, messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
        let __ai_start = std::time::Instant::now();
        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
        });

        if let Some(max_tokens) = self.config.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(temp) = self.config.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
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

        // Detect SSE streaming response (content-type: text/event-stream)
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.contains("text/event-stream") || content_type.contains("text/plain") {
            // Collect SSE chunks into full response
            return self.collect_sse_response(resp).await;
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        let choice = json["choices"]
            .get(0)
            .ok_or_else(|| LlmError::InvalidResponse("no choices".into()))?;

        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tool_calls = choice["message"]["tool_calls"].as_array().map(|calls| {
            calls
                .iter()
                .filter_map(|c| {
                    Some(ToolCall {
                        id: c["id"].as_str()?.to_string(),
                        name: c["function"]["name"].as_str()?.to_string(),
                        arguments: serde_json::from_str(
                            c["function"]["arguments"].as_str().unwrap_or("{}"),
                        )
                        .unwrap_or(serde_json::json!({})),
                    })
                })
                .collect()
        });

        let usage = json["usage"].as_object().map(|u| Usage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
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
                    latency_us: __elapsed.as_micros() as u32,
                    cost_micro_usd: 0,
                    provider_status: 200,
                    op_type: 0,
                    streaming: 0,
                    retries: 0,
                    cache_hit: 0,
                    _pad: [0; 2],
                    meta_bytes: [0; 160],
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
            finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
        })
    }

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[serde_json::Value],
    ) -> Result<ChatResponse, LlmError> {
        let __ai_start = std::time::Instant::now();
        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "tools": tools,
        });

        if let Some(max_tokens) = self.config.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(text));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        let choice = json["choices"]
            .get(0)
            .ok_or_else(|| LlmError::InvalidResponse("no choices".into()))?;

        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tool_calls = choice["message"]["tool_calls"].as_array().map(|calls| {
            calls
                .iter()
                .filter_map(|c| {
                    Some(ToolCall {
                        id: c["id"].as_str()?.to_string(),
                        name: c["function"]["name"].as_str()?.to_string(),
                        arguments: serde_json::from_str(
                            c["function"]["arguments"].as_str().unwrap_or("{}"),
                        )
                        .unwrap_or(serde_json::json!({})),
                    })
                })
                .collect()
        });

        {
            use vil_log::{ai_log, types::AiPayload};
            let __elapsed = __ai_start.elapsed();
            ai_log!(
                Info,
                AiPayload {
                    model_hash: vil_log::dict::register_str(self.model()),
                    provider_hash: vil_log::dict::register_str(self.provider_name()),
                    input_tokens: 0,
                    output_tokens: 0,
                    latency_us: __elapsed.as_micros() as u32,
                    cost_micro_usd: 0,
                    provider_status: 200,
                    op_type: 0,
                    streaming: 0,
                    retries: 0,
                    cache_hit: 0,
                    _pad: [0; 2],
                    meta_bytes: [0; 160],
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
            usage: None,
            finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
        })
    }

    fn model(&self) -> &str {
        &self.config.model
    }
    fn provider_name(&self) -> &str {
        "openai"
    }
}

impl OpenAiProvider {
    /// Collect SSE streaming response into a single ChatResponse.
    /// Handles the `data: {...}` SSE format from OpenAI-compatible endpoints.
    async fn collect_sse_response(
        &self,
        resp: reqwest::Response,
    ) -> Result<ChatResponse, LlmError> {
        let body = resp
            .text()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        let mut content = String::new();
        let mut model = self.config.model.clone();
        let mut usage = None;

        for line in body.lines() {
            let line = line.trim();
            if line == "data: [DONE]" || line.is_empty() {
                continue;
            }
            let json_str = if let Some(stripped) = line.strip_prefix("data: ") {
                stripped
            } else {
                continue;
            };

            if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(json_str) {
                // Extract model name from first chunk
                if let Some(m) = chunk["model"].as_str() {
                    model = m.to_string();
                }

                // Collect content deltas
                if let Some(choices) = chunk["choices"].as_array() {
                    for choice in choices {
                        if let Some(delta_content) = choice["delta"]["content"].as_str() {
                            content.push_str(delta_content);
                        }
                    }
                }

                // Collect usage from final chunk
                if let Some(u) = chunk["usage"].as_object() {
                    if let Some(total) = u["total_tokens"].as_u64() {
                        if total > 0 {
                            usage = Some(Usage {
                                prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                                completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0)
                                    as u32,
                                total_tokens: total as u32,
                            });
                        }
                    }
                }
            }
        }

        Ok(ChatResponse {
            content,
            model,
            tool_calls: None,
            usage,
            finish_reason: Some("stop".to_string()),
        })
    }
}

// OpenAI embedding provider
pub struct OpenAiEmbedder {
    config: OpenAiConfig,
    client: reqwest::Client,
    embedding_model: String,
    dim: usize,
}

impl OpenAiEmbedder {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        let model = model.into();
        let dim = match model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536,
        };
        Self {
            config: OpenAiConfig::new(api_key, &model),
            client: reqwest::Client::new(),
            embedding_model: model,
            dim,
        }
    }

    pub fn from_env() -> Self {
        Self::new(
            std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            std::env::var("OPENAI_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".into()),
        )
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiEmbedder {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, LlmError> {
        let __ai_start = std::time::Instant::now();
        let body = serde_json::json!({
            "model": self.embedding_model,
            "input": texts,
        });

        let resp = self
            .client
            .post(format!("{}/embeddings", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(text));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        let embeddings = json["data"]
            .as_array()
            .ok_or_else(|| LlmError::InvalidResponse("no data field".into()))?
            .iter()
            .filter_map(|item| {
                item["embedding"].as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                        .collect()
                })
            })
            .collect();

        {
            use vil_log::{ai_log, types::AiPayload};
            let __elapsed = __ai_start.elapsed();
            ai_log!(
                Info,
                AiPayload {
                    model_hash: vil_log::dict::register_str(self.model()),
                    provider_hash: vil_log::dict::register_str("openai"),
                    input_tokens: 0,
                    output_tokens: 0,
                    latency_us: __elapsed.as_micros() as u32,
                    cost_micro_usd: 0,
                    provider_status: 200,
                    op_type: 2,
                    streaming: 0,
                    retries: 0,
                    cache_hit: 0,
                    _pad: [0; 2],
                    meta_bytes: [0; 160],
                }
            );
        }

        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dim
    }
    fn model(&self) -> &str {
        &self.embedding_model
    }
}
