// 019-basic-ai-multi-model-advanced — C# SDK equivalent
// Compile: vil compile --from csharp --input 019-basic-ai-multi-model-advanced/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("AdvancedMultiModelRouterPipeline", 3086);
p.Sink("advanced_router_sink", 3086, "/route-advanced");
p.Source("advanced_router_source", "http://127.0.0.1:4545/v1/chat/completions", "sse");
p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.Compile();
