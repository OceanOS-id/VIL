// 016-basic-ai-rag-gateway — C# SDK equivalent
// Compile: vil compile --from csharp --input 016-basic-ai-rag-gateway/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("RagPipeline", 3084);
p.Sink("rag_webhook", 3084, "/rag");
p.Source("rag_sse_inference", "http://127.0.0.1:4545/v1/chat/completions", "sse");
p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.Compile();
