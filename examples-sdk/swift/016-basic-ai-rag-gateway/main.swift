// 016-basic-ai-rag-gateway — Swift SDK equivalent
// Compile: vil compile --from swift --input 016-basic-ai-rag-gateway/main.swift --release

let p = VilPipeline(name: "RagPipeline", port: 3084)
p.sink(name: "rag_webhook", port: 3084, path: "/rag")
p.source(name: "rag_sse_inference", url: "http://127.0.0.1:4545/v1/chat/completions", format: "sse")
p.route(from: "sink.trigger_out", to: "source.trigger_in", mode: "LoanWrite")
p.route(from: "source.response_data_out", to: "sink.response_data_in", mode: "LoanWrite")
p.route(from: "source.response_ctrl_out", to: "sink.response_ctrl_in", mode: "Copy")
p.compile()
