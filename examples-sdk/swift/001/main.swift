// 001 — AI Gateway (SSE Pipeline)
// Equivalent to: examples/001-basic-ai-gw-demo (Rust)
// Compile: vil compile --from swift --input 001/main.swift --release

// NOTE: Include SDK source from sdk/swift/Vil.swift when compiling.
// For single-file use: swift -I ../../sdk/swift/ main.swift

let pipeline = VilPipeline("ai-gateway", port: 3080)
    .sink("webhook", port: 3080, path: "/trigger")
    .source("inference", url: "http://localhost:4545/v1/chat/completions",
            format: "sse", jsonTap: "choices[0].delta.content", dialect: "openai")
    .route("webhook.trigger_out", to: "inference.trigger_in", mode: "LoanWrite")
    .route("inference.data_out", to: "webhook.data_in", mode: "LoanWrite")
    .route("inference.ctrl_out", to: "webhook.ctrl_in", mode: "Copy")

pipeline.compile()
