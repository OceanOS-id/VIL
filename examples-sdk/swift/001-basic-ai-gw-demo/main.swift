// 001-basic-ai-gw-demo — Swift SDK equivalent
// Compile: vil compile --from swift --input 001-basic-ai-gw-demo/main.swift --release

let p = VilPipeline(name: "DecomposedPipeline", port: 3080)
p.sink(name: "webhook_trigger", port: 3080, path: "/trigger")
p.source(name: "sse_inference", url: "http://127.0.0.1:4545/v1/chat/completions", format: "sse")
p.route(from: "sink.trigger_out", to: "source.trigger_in", mode: "LoanWrite")
p.route(from: "source.response_data_out", to: "sink.response_data_in", mode: "LoanWrite")
p.route(from: "source.response_ctrl_out", to: "sink.response_ctrl_in", mode: "Copy")
p.compile()
