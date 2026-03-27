//! Core OutputParser trait and types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed output from an LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParsedOutput {
    Json(serde_json::Value),
    Text(String),
    Structured(HashMap<String, String>),
}

/// Errors during parsing.
#[derive(Debug, Clone)]
pub enum ParseError {
    NoMatch(String),
    InvalidJson(String),
    InvalidRegex(String),
    EmptyInput,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoMatch(msg) => write!(f, "no match: {msg}"),
            Self::InvalidJson(msg) => write!(f, "invalid JSON: {msg}"),
            Self::InvalidRegex(msg) => write!(f, "invalid regex: {msg}"),
            Self::EmptyInput => write!(f, "empty input"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Trait for LLM output parsers.
pub trait OutputParser: Send + Sync {
    fn parse(&self, raw_output: &str) -> Result<ParsedOutput, ParseError>;
}
