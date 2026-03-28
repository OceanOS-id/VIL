//! SemanticRouter — the main entry point for routing queries.
//!
//! Build a router with [`SemanticRouterBuilder`], add routes, then call
//! [`SemanticRouter::route`] to dispatch a query to the best target.

use crate::classifier::KeywordClassifier;
use crate::route::Route;
use serde::{Deserialize, Serialize};
use vil_macros::VilAiDecision;
use vil_log::app_log;

/// Result of routing a query.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiDecision)]
pub struct RoutingResult {
    /// The target that the query should be sent to.
    pub target: String,
    /// Name of the route that matched (or "default" if fallback).
    pub route_name: String,
    /// Confidence of the match (`0.0` when falling back to default).
    pub confidence: f32,
    /// `true` when no route matched and the default target was used.
    pub is_default: bool,
}

/// Routes queries to specialized targets based on semantic intent.
///
/// # Example
///
/// ```
/// use vil_semantic_router::{SemanticRouter, Route};
///
/// let router = SemanticRouter::builder("general-llm")
///     .route(Route::new("math", "calculator").keywords(&["calculate", "math"]))
///     .route(Route::new("code", "code-assistant").keywords(&["code", "debug"]))
///     .build();
///
/// let result = router.route("calculate 2 + 2");
/// assert_eq!(result.target, "calculator");
/// ```
pub struct SemanticRouter {
    classifier: KeywordClassifier,
    default_target: String,
    min_confidence: f32,
}

impl SemanticRouter {
    /// Start building a `SemanticRouter` with the given default target.
    pub fn builder(default_target: &str) -> SemanticRouterBuilder {
        SemanticRouterBuilder::new(default_target)
    }

    /// Returns a reference to the underlying classifier.
    pub fn classifier(&self) -> &KeywordClassifier {
        &self.classifier
    }

    /// Returns the default target name.
    pub fn default_target(&self) -> &str {
        &self.default_target
    }

    /// Route a query to the best target.
    ///
    /// Falls back to `default_target` if no route matches or if no route
    /// reaches the minimum confidence threshold.
    pub fn route(&self, query: &str) -> RoutingResult {
        match self.classifier.classify_with_threshold(query, self.min_confidence) {
            Some(result) => {
                app_log!(Debug, "semantic_router_match", { route: result.route_name.clone(), target: result.target.clone(), confidence: result.confidence });
                RoutingResult {
                    target: result.target,
                    route_name: result.route_name,
                    confidence: result.confidence,
                    is_default: false,
                }
            }
            None => {
                app_log!(Debug, "semantic_router_default", { target: self.default_target.clone() });
                RoutingResult {
                    target: self.default_target.clone(),
                    route_name: "default".to_string(),
                    confidence: 0.0,
                    is_default: true,
                }
            }
        }
    }
}

/// Builder for [`SemanticRouter`].
pub struct SemanticRouterBuilder {
    routes: Vec<Route>,
    default_target: String,
    min_confidence: f32,
}

impl SemanticRouterBuilder {
    /// Create a builder with the given default target.
    pub fn new(default_target: &str) -> Self {
        Self {
            routes: Vec::new(),
            default_target: default_target.to_string(),
            min_confidence: 0.0,
        }
    }

    /// Add a route.
    pub fn route(mut self, route: Route) -> Self {
        self.routes.push(route);
        self
    }

    /// Add multiple routes at once.
    pub fn routes(mut self, routes: Vec<Route>) -> Self {
        self.routes.extend(routes);
        self
    }

    /// Set the minimum confidence threshold. Routes scoring below this are
    /// ignored and the default target is returned instead.
    pub fn min_confidence(mut self, c: f32) -> Self {
        self.min_confidence = c;
        self
    }

    /// Build the [`SemanticRouter`].
    pub fn build(self) -> SemanticRouter {
        SemanticRouter {
            classifier: KeywordClassifier::new(self.routes),
            default_target: self.default_target,
            min_confidence: self.min_confidence,
        }
    }
}
