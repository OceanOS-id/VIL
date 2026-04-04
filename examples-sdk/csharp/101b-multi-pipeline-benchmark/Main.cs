// 101b-multi-pipeline-benchmark — C# SDK equivalent
// Compile: vil compile --from csharp --input 101b-multi-pipeline-benchmark/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("MultiPipelineBench", 3090);
p.Sink("gateway", 3090, "/trigger");
p.Source("l_l_m_upstream", "http://127.0.0.1:4545/v1/chat/completions");
p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.Compile();
