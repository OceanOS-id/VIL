// 008-basic-credit-quality-monitor — Swift SDK equivalent
// Compile: vil compile --from swift --input 008-basic-credit-quality-monitor/main.swift --release

let p = VilPipeline(name: "CreditQualityMonitorPipeline", port: 3082)
p.sink(name: "quality_monitor_sink", port: 3082, path: "/quality-check")
p.source(name: "quality_credit_source", format: "json")
p.route(from: "sink.trigger_out", to: "source.trigger_in", mode: "LoanWrite")
p.route(from: "source.response_data_out", to: "sink.response_data_in", mode: "LoanWrite")
p.route(from: "source.response_ctrl_out", to: "sink.response_ctrl_in", mode: "Copy")
p.compile()
