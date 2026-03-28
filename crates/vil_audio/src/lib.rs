//! # VIL Audio Transcription (I03)
//!
//! Infrastructure and traits for audio-to-text transcription.
//! This crate provides the trait definitions, result types, and audio format
//! detection. Actual transcription backends (Whisper, etc.) plug in via the
//! `Transcriber` trait.
//!
//! ## Quick Start
//!
//! ```rust
//! use vil_audio::{AudioFormat, detect_format, TranscriptConfig};
//!
//! let wav_header = b"RIFF\x00\x00\x00\x00WAVEfmt ";
//! assert_eq!(detect_format(wav_header), AudioFormat::Wav);
//!
//! let config = TranscriptConfig::new().language("en").timestamps(true);
//! ```

pub mod config;
pub mod format;
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod result;
pub mod semantic;
pub mod transcriber;

pub use config::TranscriptConfig;
pub use format::{detect_format, AudioFormat};
pub use plugin::AudioPlugin;
pub use result::{Segment, Transcript};
pub use semantic::{AudioEvent, AudioFault, AudioFaultType, AudioState};
pub use transcriber::{NoopTranscriber, TranscribeError, Transcriber};
