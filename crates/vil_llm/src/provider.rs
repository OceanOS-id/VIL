use async_trait::async_trait;
use crate::message::{ChatMessage, ChatResponse, LlmError};

/// Core LLM provider trait — chat completion + streaming.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Chat completion (non-streaming).
    async fn chat(&self, messages: &[ChatMessage]) -> Result<ChatResponse, LlmError>;

    /// Chat completion with tool definitions.
    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[serde_json::Value],
    ) -> Result<ChatResponse, LlmError> {
        // Default: ignore tools, just chat
        let _ = tools;
        self.chat(messages).await
    }

    /// Model identifier.
    fn model(&self) -> &str;

    /// Provider name (e.g., "openai", "anthropic", "ollama").
    fn provider_name(&self) -> &str;
}

/// Embedding provider trait — text to vector.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embeddings for the given texts.
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, LlmError>;

    /// Embedding dimension.
    fn dimension(&self) -> usize;

    /// Model identifier.
    fn model(&self) -> &str;
}
