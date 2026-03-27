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

pub mod provider;
pub mod message;
pub mod openai;
pub mod anthropic;
pub mod ollama;
pub mod router;
pub mod semantic;
pub mod extractors;
pub mod pipeline;
pub mod handlers;
pub mod plugin;

pub use provider::{LlmProvider, EmbeddingProvider};
pub use message::{ChatMessage, ChatResponse, Role, ToolCall, Usage, LlmError};
pub use openai::{OpenAiProvider, OpenAiConfig, OpenAiEmbedder};
pub use anthropic::{AnthropicProvider, AnthropicConfig};
pub use ollama::{OllamaProvider, OllamaConfig};
pub use router::{LlmRouter, RouterStrategy};
pub use plugin::LlmPlugin;
pub use extractors::{Llm, Embedder};
pub use semantic::{LlmResponseEvent, LlmStreamChunkEvent, LlmFault, LlmFaultType, LlmUsageState};
