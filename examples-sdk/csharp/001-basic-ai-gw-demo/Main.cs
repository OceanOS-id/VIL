// 001-basic-ai-gw-demo — C# SDK equivalent
// Compile: vil compile --from csharp --input 001-basic-ai-gw-demo/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("DecomposedPipeline", 3080);
p.Sink("webhook_trigger", 3080, "/trigger");
p.Source("sse_inference", "http://127.0.0.1:4545/v1/chat/completions", "sse");
p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.Compile();
