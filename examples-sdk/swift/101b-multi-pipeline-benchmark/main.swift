// 101b-multi-pipeline-benchmark — Swift SDK equivalent
// Compile: vil compile --from swift --input 101b-multi-pipeline-benchmark/main.swift --release

let p = VilPipeline(name: "MultiPipelineBench", port: 3090)
p.sink(name: "gateway", port: 3090, path: "/trigger")
p.source(name: "l_l_m_upstream", url: "http://127.0.0.1:4545/v1/chat/completions")
p.route(from: "sink.trigger_out", to: "source.trigger_in", mode: "LoanWrite")
p.route(from: "source.response_data_out", to: "sink.response_data_in", mode: "LoanWrite")
p.route(from: "source.response_ctrl_out", to: "sink.response_ctrl_in", mode: "Copy")
p.compile()
