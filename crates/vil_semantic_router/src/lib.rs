//! # VIL Semantic Router
//!
//! Routes queries to specialized models, pipelines, or services based on
//! semantic intent classification.
//!
//! ## Quick Start
//!
//! ```rust
//! use vil_semantic_router::{SemanticRouter, Route, config};
//!
//! // Option 1: Build manually
//! let router = SemanticRouter::builder("general-llm")
//!     .route(Route::new("math", "calculator").keywords(&["calculate", "math", "sum"]))
//!     .route(Route::new("code", "code-assistant").keywords(&["code", "debug", "function"]))
//!     .min_confidence(0.1)
//!     .build();
//!
//! let result = router.route("please calculate the sum of 1 and 2");
//! assert_eq!(result.target, "calculator");
//! assert!(!result.is_default);
//!
//! // Option 2: Use pre-built AI platform routes
//! let router = SemanticRouter::builder("general-llm")
//!     .routes(config::ai_platform_routes())
//!     .build();
//!
//! let result = router.route("translate this to french");
//! assert_eq!(result.target, "translator");
//! ```

pub mod classifier;
pub mod config;
pub mod route;
pub mod router;

// Re-exports for ergonomic usage.
pub use classifier::{ClassificationResult, KeywordClassifier};
pub use config::ai_platform_routes;
pub use route::Route;
pub use router::{RoutingResult, SemanticRouter, SemanticRouterBuilder};

pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::SemanticRouterPlugin;
pub use semantic::{RouteEvent, RouteFault, RouterState};

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // 1. Route builder pattern
    // ---------------------------------------------------------------
    #[test]
    fn test_route_builder() {
        let route = Route::new("math", "calculator")
            .keywords(&["calculate", "math"])
            .examples(&["what is 2+2?"])
            .priority(5)
            .description("Math stuff");

        assert_eq!(route.name, "math");
        assert_eq!(route.target, "calculator");
        assert_eq!(route.keywords, vec!["calculate", "math"]);
        assert_eq!(route.examples, vec!["what is 2+2?"]);
        assert_eq!(route.priority, 5);
        assert_eq!(route.description, "Math stuff");
    }

    // ---------------------------------------------------------------
    // 2. Keyword classification — single match
    // ---------------------------------------------------------------
    #[test]
    fn test_classify_single_match() {
        let routes = vec![
            Route::new("math", "calculator").keywords(&["calculate", "math", "sum"]),
            Route::new("code", "code-assistant").keywords(&["code", "debug"]),
        ];
        let classifier = KeywordClassifier::new(routes);

        let result = classifier.classify("please calculate the sum").unwrap();
        assert_eq!(result.route_name, "math");
        assert_eq!(result.target, "calculator");
        assert!(result.confidence > 0.5);
        assert!(result.matched_keywords.contains(&"calculate".to_string()));
        assert!(result.matched_keywords.contains(&"sum".to_string()));
    }

    // ---------------------------------------------------------------
    // 3. Keyword classification — multiple routes match, highest wins
    // ---------------------------------------------------------------
    #[test]
    fn test_classify_multiple_match_highest_confidence_wins() {
        let routes = vec![
            Route::new("search", "rag").keywords(&["search", "find", "lookup", "document"]),
            Route::new("code", "code-assistant").keywords(&["code", "debug"]),
        ];
        let classifier = KeywordClassifier::new(routes);

        // "debug" matches code (1/2 = 0.5), "find" matches search (1/4 = 0.25)
        // But "code debug" matches code with 2/2 = 1.0
        let result = classifier.classify("code debug this function").unwrap();
        assert_eq!(result.route_name, "code");
        assert!((result.confidence - 1.0).abs() < f32::EPSILON);
    }

    // ---------------------------------------------------------------
    // 4. Keyword classification — no match
    // ---------------------------------------------------------------
    #[test]
    fn test_classify_no_match() {
        let routes = vec![
            Route::new("math", "calculator").keywords(&["calculate", "math"]),
        ];
        let classifier = KeywordClassifier::new(routes);

        let result = classifier.classify("tell me a joke");
        assert!(result.is_none());
    }

    // ---------------------------------------------------------------
    // 5. Confidence scoring
    // ---------------------------------------------------------------
    #[test]
    fn test_confidence_scoring() {
        let routes = vec![
            Route::new("math", "calculator").keywords(&["calculate", "math", "sum", "multiply"]),
        ];
        let classifier = KeywordClassifier::new(routes);

        // 1 out of 4 keywords => 0.25
        let r1 = classifier.classify("calculate something").unwrap();
        assert!((r1.confidence - 0.25).abs() < f32::EPSILON);

        // 2 out of 4 keywords => 0.5
        let r2 = classifier.classify("calculate the sum").unwrap();
        assert!((r2.confidence - 0.5).abs() < f32::EPSILON);

        // threshold filters out low confidence
        let r3 = classifier.classify_with_threshold("calculate something", 0.3);
        assert!(r3.is_none());
    }

    // ---------------------------------------------------------------
    // 6. Priority ordering (lower priority number wins on tie)
    // ---------------------------------------------------------------
    #[test]
    fn test_priority_ordering() {
        let routes = vec![
            Route::new("general", "general-model")
                .keywords(&["help"])
                .priority(100),
            Route::new("support", "support-bot")
                .keywords(&["help"])
                .priority(10),
        ];
        let classifier = KeywordClassifier::new(routes);

        // Both match with equal confidence; support has lower priority number => wins.
        let result = classifier.classify("help me").unwrap();
        assert_eq!(result.route_name, "support");
        assert_eq!(result.target, "support-bot");
    }

    // ---------------------------------------------------------------
    // 7. Default fallback via SemanticRouter
    // ---------------------------------------------------------------
    #[test]
    fn test_default_fallback() {
        let router = SemanticRouter::builder("general-llm")
            .route(Route::new("math", "calculator").keywords(&["calculate"]))
            .build();

        let result = router.route("tell me a joke");
        assert!(result.is_default);
        assert_eq!(result.target, "general-llm");
        assert_eq!(result.route_name, "default");
        assert!((result.confidence - 0.0).abs() < f32::EPSILON);
    }

    // ---------------------------------------------------------------
    // 8. Pre-built ai_platform_routes
    // ---------------------------------------------------------------
    #[test]
    fn test_prebuilt_routes() {
        let routes = ai_platform_routes();
        assert!(routes.len() >= 6);

        let names: Vec<&str> = routes.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"math"));
        assert!(names.contains(&"code"));
        assert!(names.contains(&"search"));
        assert!(names.contains(&"summarize"));
        assert!(names.contains(&"translate"));
        assert!(names.contains(&"creative"));
    }

    // ---------------------------------------------------------------
    // 9. SemanticRouter end-to-end
    // ---------------------------------------------------------------
    #[test]
    fn test_router_end_to_end() {
        let router = SemanticRouter::builder("general-llm")
            .routes(ai_platform_routes())
            .min_confidence(0.1)
            .build();

        let math = router.route("calculate the sum of 10 and 20");
        assert_eq!(math.target, "calculator");
        assert!(!math.is_default);

        let code = router.route("debug this function please");
        assert_eq!(code.target, "code-assistant");

        let translate = router.route("translate this to french");
        assert_eq!(translate.target, "translator");

        let fallback = router.route("hello world how are you");
        assert!(fallback.is_default);
        assert_eq!(fallback.target, "general-llm");
    }

    // ---------------------------------------------------------------
    // 10. Case insensitivity
    // ---------------------------------------------------------------
    #[test]
    fn test_case_insensitivity() {
        let router = SemanticRouter::builder("general-llm")
            .route(Route::new("math", "calculator").keywords(&["Calculate", "MATH"]))
            .build();

        let result = router.route("CALCULATE the MATH problem");
        assert_eq!(result.target, "calculator");
        assert!(!result.is_default);

        let result2 = router.route("calculate the math problem");
        assert_eq!(result2.target, "calculator");
    }

    // ---------------------------------------------------------------
    // 11. Multi-word keyword matching
    // ---------------------------------------------------------------
    #[test]
    fn test_multi_word_keywords() {
        let router = SemanticRouter::builder("general-llm")
            .route(Route::new("search", "rag").keywords(&["what is", "who is"]))
            .build();

        let result = router.route("what is the meaning of life?");
        assert_eq!(result.target, "rag");
        assert!(!result.is_default);
    }

    // ---------------------------------------------------------------
    // 12. Min confidence threshold on router
    // ---------------------------------------------------------------
    #[test]
    fn test_min_confidence_threshold_on_router() {
        let router = SemanticRouter::builder("general-llm")
            .route(
                Route::new("math", "calculator")
                    .keywords(&["calculate", "math", "sum", "multiply", "divide"]),
            )
            .min_confidence(0.5)
            .build();

        // Only 1 of 5 keywords matches => 0.2 < 0.5 threshold => fallback
        let result = router.route("calculate something random");
        assert!(result.is_default);

        // 3 of 5 keywords => 0.6 >= 0.5 => match
        let result2 = router.route("calculate the sum and multiply");
        assert!(!result2.is_default);
        assert_eq!(result2.target, "calculator");
    }
}
