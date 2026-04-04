// 007-basic-credit-npl-filter — Swift SDK equivalent
// Compile: vil compile --from swift --input 007-basic-credit-npl-filter/main.swift --release

let p = VilPipeline(name: "NplFilterPipeline", port: 3081)
p.sink(name: "npl_filter_sink", port: 3081, path: "/filter-npl")
p.source(name: "npl_credit_source", format: "json")
p.route(from: "sink.trigger_out", to: "source.trigger_in", mode: "LoanWrite")
p.route(from: "source.response_data_out", to: "sink.response_data_in", mode: "LoanWrite")
p.route(from: "source.response_ctrl_out", to: "sink.response_ctrl_in", mode: "Copy")
p.compile()
