//! Route definitions for semantic routing.
//!
//! A [`Route`] maps a named intent to a target (model, pipeline, service).

use serde::{Deserialize, Serialize};

/// A route maps an intent to a target.
///
/// Routes are matched against incoming queries using keyword matching.
/// When multiple routes match, the one with the highest confidence wins;
/// ties are broken by priority (lower number = higher priority).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// Unique name for this route (e.g. "math", "code", "search").
    pub name: String,
    /// Human-readable description of what this route handles.
    pub description: String,
    /// Keywords that trigger this route (matched case-insensitively).
    pub keywords: Vec<String>,
    /// Target identifier (model name, pipeline name, service URL, etc.).
    pub target: String,
    /// Priority: lower = higher priority when multiple routes match with equal confidence.
    pub priority: u32,
    /// Optional example queries for documentation and future embedding-based matching.
    pub examples: Vec<String>,
}

impl Route {
    /// Create a new route with the given name and target.
    ///
    /// Defaults: priority=100, no keywords, no examples, empty description.
    pub fn new(name: &str, target: &str) -> Self {
        Self {
            name: name.to_string(),
            description: String::new(),
            keywords: Vec::new(),
            target: target.to_string(),
            priority: 100,
            examples: Vec::new(),
        }
    }

    /// Set keywords that trigger this route.
    pub fn keywords(mut self, kw: &[&str]) -> Self {
        self.keywords = kw.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set example queries for this route.
    pub fn examples(mut self, ex: &[&str]) -> Self {
        self.examples = ex.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set priority (lower = higher priority).
    pub fn priority(mut self, p: u32) -> Self {
        self.priority = p;
        self
    }

    /// Set a human-readable description.
    pub fn description(mut self, d: &str) -> Self {
        self.description = d.to_string();
        self
    }
}
