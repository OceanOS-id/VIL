use std::sync::Arc;

use crate::provider::{EmbedError, EmbedProvider};

/// Concurrent batch embedder that splits a large list of texts into
/// optimally-sized chunks and processes them in parallel.
pub struct BatchEmbedder {
    provider: Arc<dyn EmbedProvider>,
    max_batch_size: usize,
    max_concurrent: usize,
}

impl BatchEmbedder {
    /// Create a new `BatchEmbedder` wrapping the given provider.
    ///
    /// Defaults:
    /// - `max_batch_size`: provider's `max_batch_size()`
    /// - `max_concurrent`: 4
    pub fn new(provider: Arc<dyn EmbedProvider>) -> Self {
        let max_batch_size = provider.max_batch_size();
        Self {
            provider,
            max_batch_size,
            max_concurrent: 4,
        }
    }

    /// Override the maximum number of texts per API call.
    pub fn max_batch_size(mut self, n: usize) -> Self {
        self.max_batch_size = n;
        self
    }

    /// Override the maximum number of concurrent API calls.
    pub fn max_concurrent(mut self, n: usize) -> Self {
        self.max_concurrent = n;
        self
    }

    /// Embed a large list of texts by splitting into optimal batches
    /// and processing concurrently.
    pub async fn embed_all(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let chunks: Vec<Vec<String>> = texts
            .chunks(self.max_batch_size)
            .map(|c| c.to_vec())
            .collect();

        let total_chunks = chunks.len();
        let mut results: Vec<(usize, Vec<Vec<f32>>)> = Vec::with_capacity(total_chunks);

        // Process chunks in windows of max_concurrent using JoinSet.
        for window_start in (0..total_chunks).step_by(self.max_concurrent) {
            let window_end = (window_start + self.max_concurrent).min(total_chunks);
            let mut join_set = tokio::task::JoinSet::new();

            for idx in window_start..window_end {
                let provider = Arc::clone(&self.provider);
                let chunk = chunks[idx].clone();
                join_set.spawn(async move {
                    let vecs = provider.embed_batch(&chunk).await?;
                    Ok::<(usize, Vec<Vec<f32>>), EmbedError>((idx, vecs))
                });
            }

            while let Some(result) = join_set.join_next().await {
                let (idx, vecs) = result
                    .map_err(|e| EmbedError::RequestFailed(format!("task join error: {e}")))?
                    ?;
                results.push((idx, vecs));
            }
        }

        // Sort by chunk index and flatten.
        results.sort_by_key(|(idx, _)| *idx);
        let flat: Vec<Vec<f32>> = results.into_iter().flat_map(|(_, vecs)| vecs).collect();

        Ok(flat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A example provider that returns a fixed-dimension zero vector for each input
    /// and tracks how many batch calls were made.
    struct MockProvider {
        dim: usize,
        batch_size: usize,
        call_count: AtomicUsize,
    }

    impl MockProvider {
        fn new(dim: usize, batch_size: usize) -> Self {
            Self {
                dim,
                batch_size,
                call_count: AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl EmbedProvider for MockProvider {
        async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(texts
                .iter()
                .enumerate()
                .map(|(i, _)| vec![i as f32; self.dim])
                .collect())
        }

        fn dimension(&self) -> usize {
            self.dim
        }

        fn model_name(&self) -> &str {
            "noop-model"
        }

        fn max_batch_size(&self) -> usize {
            self.batch_size
        }
    }

    #[tokio::test]
    async fn batch_splitting_100_texts_into_2_batches() {
        let provider = Arc::new(MockProvider::new(4, 50));
        let embedder = BatchEmbedder::new(Arc::clone(&provider) as Arc<dyn EmbedProvider>);

        let texts: Vec<String> = (0..100).map(|i| format!("text-{i}")).collect();
        let results = embedder.embed_all(&texts).await.unwrap();

        assert_eq!(results.len(), 100);
        assert_eq!(provider.call_count(), 2);
        // Each vector should have the correct dimension.
        for v in &results {
            assert_eq!(v.len(), 4);
        }
    }

    #[tokio::test]
    async fn batch_splitting_with_remainder() {
        let provider = Arc::new(MockProvider::new(8, 30));
        let embedder = BatchEmbedder::new(Arc::clone(&provider) as Arc<dyn EmbedProvider>);

        let texts: Vec<String> = (0..75).map(|i| format!("text-{i}")).collect();
        let results = embedder.embed_all(&texts).await.unwrap();

        assert_eq!(results.len(), 75);
        // 75 / 30 = 2 full batches + 1 remainder = 3 calls
        assert_eq!(provider.call_count(), 3);
    }

    #[tokio::test]
    async fn empty_input_returns_empty() {
        let provider = Arc::new(MockProvider::new(4, 50));
        let embedder = BatchEmbedder::new(Arc::clone(&provider) as Arc<dyn EmbedProvider>);

        let results = embedder.embed_all(&[]).await.unwrap();
        assert!(results.is_empty());
        assert_eq!(provider.call_count(), 0);
    }

    #[tokio::test]
    async fn single_text_single_batch() {
        let provider = Arc::new(MockProvider::new(4, 50));
        let embedder = BatchEmbedder::new(Arc::clone(&provider) as Arc<dyn EmbedProvider>);

        let texts = vec!["hello".to_string()];
        let results = embedder.embed_all(&texts).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(provider.call_count(), 1);
    }

    #[tokio::test]
    async fn respects_max_concurrent() {
        let provider = Arc::new(MockProvider::new(4, 10));
        let embedder = BatchEmbedder::new(Arc::clone(&provider) as Arc<dyn EmbedProvider>)
            .max_batch_size(10)
            .max_concurrent(2);

        let texts: Vec<String> = (0..50).map(|i| format!("text-{i}")).collect();
        let results = embedder.embed_all(&texts).await.unwrap();

        assert_eq!(results.len(), 50);
        // 50 / 10 = 5 batch calls total
        assert_eq!(provider.call_count(), 5);
    }
}
