use crate::message::*;
use crate::provider::LlmProvider;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use vil_log::app_log;

#[derive(Debug, Clone, Copy)]
pub enum RouterStrategy {
    /// Always use the first provider
    Primary,
    /// Round-robin across providers
    RoundRobin,
    /// Try first, fallback to second on error
    Fallback,
}

pub struct LlmRouter {
    providers: Vec<Arc<dyn LlmProvider>>,
    strategy: RouterStrategy,
    counter: AtomicUsize,
}

impl LlmRouter {
    pub fn new(strategy: RouterStrategy) -> Self {
        Self {
            providers: Vec::new(),
            strategy,
            counter: AtomicUsize::new(0),
        }
    }

    pub fn add_provider(mut self, provider: Arc<dyn LlmProvider>) -> Self {
        self.providers.push(provider);
        self
    }
}

#[async_trait]
impl LlmProvider for LlmRouter {
    async fn chat(&self, messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
        if self.providers.is_empty() {
            return Err(LlmError::ModelNotFound("no providers configured".into()));
        }

        match self.strategy {
            RouterStrategy::Primary => {
                self.providers
                    .first()
                    .ok_or(LlmError::ModelNotFound("no providers".into()))?
                    .chat(messages)
                    .await
            }
            RouterStrategy::RoundRobin => {
                let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.providers.len();
                self.providers[idx].chat(messages).await
            }
            RouterStrategy::Fallback => {
                for (i, provider) in self.providers.iter().enumerate() {
                    match provider.chat(messages).await {
                        Ok(resp) => return Ok(resp),
                        Err(e) => {
                            if i == self.providers.len() - 1 {
                                return Err(e);
                            }
                            app_log!(Warn, "llm_router_fallback", { provider: provider.provider_name().to_string(), error: e.to_string() });
                        }
                    }
                }
                Err(LlmError::RequestFailed("all providers failed".into()))
            }
        }
    }

    fn model(&self) -> &str {
        self.providers
            .first()
            .map(|p| p.model())
            .unwrap_or("router")
    }

    fn provider_name(&self) -> &str {
        "router"
    }
}
