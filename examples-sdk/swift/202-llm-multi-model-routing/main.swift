// 202-llm-multi-model-routing — Swift SDK equivalent
let p = VilPipeline(name: "MultiModelPipeline_GPT4", port: 8080)
p.route(from: "sink.trigger_out", to: "source_gpt4.trigger_in", mode: "LoanWrite")
p.route(from: "source_gpt4.response_data_out", to: "sink.response_data_in", mode: "LoanWrite")
p.route(from: "source_gpt4.response_ctrl_out", to: "sink.response_ctrl_in", mode: "Copy")
p.compile()
