//! Intent classification using keyword matching and scoring.
//!
//! The [`KeywordClassifier`] scores each route against a query by counting
//! how many of the route's keywords appear in the query text. Confidence
//! is the fraction of keywords matched.

use crate::route::Route;
use serde::{Deserialize, Serialize};

/// Result of classifying a query against the route table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// Name of the matched route.
    pub route_name: String,
    /// Target identifier from the matched route.
    pub target: String,
    /// Confidence score in `[0.0, 1.0]` — fraction of keywords matched.
    pub confidence: f32,
    /// The keywords that were found in the query.
    pub matched_keywords: Vec<String>,
}

/// Classify query intent using keyword matching and scoring.
///
/// For each route, the classifier counts how many of the route's keywords
/// appear in the (lowercased) query. The confidence is `matched / total`.
/// When multiple routes match, the one with the highest confidence wins;
/// ties are broken by the route's priority (lower number wins).
pub struct KeywordClassifier {
    routes: Vec<Route>,
}

impl KeywordClassifier {
    /// Create a new classifier with the given routes.
    pub fn new(routes: Vec<Route>) -> Self {
        Self { routes }
    }

    /// Classify a query. Returns the best matching route, or `None` if no
    /// route has any keyword match.
    pub fn classify(&self, query: &str) -> Option<ClassificationResult> {
        self.classify_with_threshold(query, 0.0)
    }

    /// Classify a query with a minimum confidence threshold.
    ///
    /// Returns `None` if no route reaches `min_confidence`.
    pub fn classify_with_threshold(
        &self,
        query: &str,
        min_confidence: f32,
    ) -> Option<ClassificationResult> {
        let query_lower = query.to_lowercase();

        let mut best: Option<(ClassificationResult, u32)> = None;

        for route in &self.routes {
            if route.keywords.is_empty() {
                continue;
            }

            let matched: Vec<String> = route
                .keywords
                .iter()
                .filter(|kw| {
                    let kw_lower = kw.to_lowercase();
                    contains_word(&query_lower, &kw_lower)
                })
                .cloned()
                .collect();

            if matched.is_empty() {
                continue;
            }

            let confidence = matched.len() as f32 / route.keywords.len() as f32;

            if confidence < min_confidence {
                continue;
            }

            let is_better = match &best {
                None => true,
                Some((prev, prev_priority)) => {
                    if confidence > prev.confidence {
                        true
                    } else if (confidence - prev.confidence).abs() < f32::EPSILON
                        && route.priority < *prev_priority
                    {
                        true
                    } else {
                        false
                    }
                }
            };

            if is_better {
                best = Some((
                    ClassificationResult {
                        route_name: route.name.clone(),
                        target: route.target.clone(),
                        confidence,
                        matched_keywords: matched,
                    },
                    route.priority,
                ));
            }
        }

        best.map(|(result, _)| result)
    }

    /// Return a reference to the routes owned by this classifier.
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }
}

/// Check if `haystack` contains `needle` as a word or phrase.
///
/// A keyword like "what is" should match inside "what is the meaning of life".
/// Single words are matched on word boundaries; multi-word phrases are matched
/// as substrings (both lowercased by the caller).
fn contains_word(haystack: &str, needle: &str) -> bool {
    if needle.contains(' ') {
        // Multi-word phrase: substring match is sufficient.
        return haystack.contains(needle);
    }

    // Single word: check word boundaries.
    for word in haystack.split(|c: char| !c.is_alphanumeric()) {
        if word == needle {
            return true;
        }
    }
    false
}
