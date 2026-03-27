//! Pre-built route configurations for common use cases.

use crate::route::Route;

/// Pre-built routes for an AI platform that needs to dispatch queries to
/// specialized models/pipelines (calculator, code assistant, RAG, etc.).
pub fn ai_platform_routes() -> Vec<Route> {
    vec![
        Route::new("math", "calculator")
            .keywords(&[
                "calculate", "compute", "math", "equation", "sum", "multiply", "divide", "percent",
            ])
            .description("Mathematical calculations")
            .priority(10),
        Route::new("code", "code-assistant")
            .keywords(&[
                "code", "program", "function", "debug", "compile", "syntax", "algorithm",
                "implement",
            ])
            .description("Code generation and debugging")
            .priority(20),
        Route::new("search", "rag-pipeline")
            .keywords(&[
                "search", "find", "lookup", "what is", "who is", "when did", "document",
                "knowledge",
            ])
            .description("Knowledge base search")
            .priority(30),
        Route::new("summarize", "summarizer")
            .keywords(&["summarize", "summary", "tldr", "brief", "condense", "key points"])
            .description("Text summarization")
            .priority(40),
        Route::new("translate", "translator")
            .keywords(&[
                "translate", "translation", "convert to", "in english", "in spanish", "in french",
            ])
            .description("Language translation")
            .priority(50),
        Route::new("creative", "creative-writer")
            .keywords(&["write", "story", "poem", "essay", "creative", "compose", "draft"])
            .description("Creative writing")
            .priority(60),
    ]
}
