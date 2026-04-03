// 001 — AI Gateway (SSE Pipeline)
// Equivalent to: examples/001-basic-ai-gw-demo (Rust)
// Compile: vil compile --from kotlin --input 001/main.kt --release

// NOTE: When using `kotlin` script runner, paste Vil SDK classes above or use @file:Import
// For standalone use, include the SDK source from sdk/kotlin/vil.kt

fun main() {
    val pipeline = VilPipeline("ai-gateway", 3080)
        .sink("webhook", 3080, "/trigger")
        .source("inference", "http://localhost:4545/v1/chat/completions",
            format = "sse", dialect = "openai", jsonTap = "choices[0].delta.content")
        .route("webhook.trigger_out", "inference.trigger_in", "LoanWrite")
        .route("inference.data_out", "webhook.data_in", "LoanWrite")
        .route("inference.ctrl_out", "webhook.ctrl_in", "Copy")

    pipeline.compile()
}
