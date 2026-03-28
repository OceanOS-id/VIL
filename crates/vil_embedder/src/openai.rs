use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::provider::{EmbedError, EmbedProvider};

/// Configuration for the OpenAI embeddings provider.
#[derive(Debug, Clone)]
pub struct OpenAiEmbedder {
    api_key: String,
    model: String,
    base_url: String,
    dimension: usize,
    client: reqwest::Client,
}

impl OpenAiEmbedder {
    /// Create a new OpenAI embedder with the given API key.
    ///
    /// Defaults:
    /// - model: `text-embedding-3-small`
    /// - base_url: `https://api.openai.com`
    /// - dimension: `1536`
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "text-embedding-3-small".to_string(),
            base_url: "https://api.openai.com".to_string(),
            dimension: 1536,
            client: reqwest::Client::new(),
        }
    }

    /// Override the model name.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Override the base URL (useful for proxies or Azure endpoints).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Override the expected embedding dimension.
    pub fn with_dimension(mut self, dim: usize) -> Self {
        self.dimension = dim;
        self
    }
}

// ── OpenAI API request / response types ──────────────────────────────

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: &'a [String],
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    #[allow(dead_code)]
    index: usize,
}

#[async_trait]
impl EmbedProvider for OpenAiEmbedder {
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let url = format!("{}/v1/embeddings", self.base_url);
        let body = EmbeddingRequest {
            model: &self.model,
            input: texts,
        };

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| EmbedError::RequestFailed(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(EmbedError::RateLimited);
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_else(|_| "<unreadable>".into());
            return Err(EmbedError::RequestFailed(format!(
                "HTTP {status}: {body_text}"
            )));
        }

        let embedding_resp: EmbeddingResponse = resp
            .json()
            .await
            .map_err(|e| EmbedError::RequestFailed(format!("JSON decode: {e}")))?;

        // Sort by index to guarantee ordering matches input order.
        let mut data = embedding_resp.data;
        data.sort_by_key(|d| d.index);

        let vectors: Vec<Vec<f32>> = data.into_iter().map(|d| d.embedding).collect();

        // Validate dimension on the first vector.
        if let Some(first) = vectors.first() {
            if first.len() != self.dimension {
                return Err(EmbedError::DimensionMismatch {
                    expected: self.dimension,
                    got: first.len(),
                });
            }
        }

        Ok(vectors)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_construction() {
        let embedder = OpenAiEmbedder::new("sk-test-key");
        assert_eq!(embedder.model_name(), "text-embedding-3-small");
        assert_eq!(embedder.dimension(), 1536);
        assert_eq!(embedder.base_url, "https://api.openai.com");
    }

    #[test]
    fn builder_overrides() {
        let embedder = OpenAiEmbedder::new("sk-test-key")
            .model("text-embedding-3-large")
            .base_url("https://my-proxy.example.com")
            .with_dimension(3072);

        assert_eq!(embedder.model_name(), "text-embedding-3-large");
        assert_eq!(embedder.dimension(), 3072);
        assert_eq!(embedder.base_url, "https://my-proxy.example.com");
    }

    #[test]
    fn max_batch_size_default() {
        let embedder = OpenAiEmbedder::new("sk-test-key");
        assert_eq!(embedder.max_batch_size(), 100);
    }
}
