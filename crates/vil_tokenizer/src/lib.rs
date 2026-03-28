//! VIL Tokenizer Engine
//!
//! Native Rust BPE (Byte-Pair Encoding) tokenizer for:
//! - Token counting (how many tokens in this text?)
//! - Text truncation (cut to N tokens without breaking words)
//! - Encoding/decoding (text <-> token IDs)
//!
//! Compatible with OpenAI tiktoken and Llama sentencepiece vocabularies.

pub mod bpe;
pub mod counter;
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;
pub mod truncate;
pub mod vocab;

pub use bpe::BpeTokenizer;
pub use counter::TokenCounter;
pub use plugin::TokenizerPlugin;
pub use semantic::{TokenizeEvent, TokenizeFault, TokenizerState};
pub use truncate::{truncate_to_tokens, TruncateStrategy};
pub use vocab::{VocabSource, Vocabulary};
