//! VIL LLM Plugin — multi-provider LLM abstraction.
//!
//! Provides `LlmProvider` trait and implementations for OpenAI, Anthropic, Ollama.
//! Registers as a VIL plugin via `VilApp::new("app").plugin(LlmPlugin::new())`.
//!
//! # Architecture
//!
//! - **Layer 1 (Core logic):** `provider`, `message`, `openai`, `anthropic`, `ollama`, `router`
//! - **Layer 2 (VIL integration):** `plugin`, `handlers`, `semantic`, `extractors`
//!
//! # Plugin endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | POST | /api/llm/chat | Chat completion |
//! | POST | /api/llm/embed | Text embedding |
//! | GET | /api/llm/models | List available models |

pub mod anthropic;
pub mod extractors;
pub mod handlers;
pub mod message;
pub mod ollama;
pub mod openai;
pub mod pipeline;
pub mod plugin;
pub mod provider;
pub mod router;
pub mod semantic;

pub use anthropic::{AnthropicConfig, AnthropicProvider};
pub use extractors::{Embedder, Llm};
pub use message::{ChatMessage, ChatResponse, LlmError, Role, ToolCall, Usage};
pub use ollama::{OllamaConfig, OllamaProvider};
pub use openai::{OpenAiConfig, OpenAiEmbedder, OpenAiProvider};
pub use plugin::LlmPlugin;
pub use provider::{EmbeddingProvider, LlmProvider};
pub use router::{LlmRouter, RouterStrategy};
pub use semantic::{LlmFault, LlmFaultType, LlmResponseEvent, LlmStreamChunkEvent, LlmUsageState};
