// 019-basic-ai-multi-model-advanced — Swift SDK equivalent
// Compile: vil compile --from swift --input 019-basic-ai-multi-model-advanced/main.swift --release

let p = VilPipeline(name: "AdvancedMultiModelRouterPipeline", port: 3086)
p.sink(name: "advanced_router_sink", port: 3086, path: "/route-advanced")
p.source(name: "advanced_router_source", url: "http://127.0.0.1:4545/v1/chat/completions", format: "sse")
p.route(from: "sink.trigger_out", to: "source.trigger_in", mode: "LoanWrite")
p.route(from: "source.response_data_out", to: "sink.response_data_in", mode: "LoanWrite")
p.route(from: "source.response_ctrl_out", to: "sink.response_ctrl_in", mode: "Copy")
p.compile()
