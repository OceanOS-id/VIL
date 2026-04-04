// 205-llm-chunked-summarizer — Swift SDK equivalent
let p = VilPipeline(name: "ChunkedSummarizerPipeline", port: 8080)
p.route(from: "sink.trigger_out", to: "source_summarize.trigger_in", mode: "LoanWrite")
p.route(from: "source_summarize.response_data_out", to: "sink.response_data_in", mode: "LoanWrite")
p.route(from: "source_summarize.response_ctrl_out", to: "sink.response_ctrl_in", mode: "Copy")
p.compile()
