// 202-llm-multi-model-routing — C# SDK equivalent
var p = new VilPipeline("MultiModelPipeline_GPT4", 8080);
p.Route("sink.trigger_out", "source_gpt4.trigger_in", "LoanWrite");
p.Route("source_gpt4.response_data_out", "sink.response_data_in", "LoanWrite");
p.Route("source_gpt4.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.Compile();
