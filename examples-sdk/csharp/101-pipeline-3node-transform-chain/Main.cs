// 101-pipeline-3node-transform-chain — C# SDK equivalent
// Compile: vil compile --from csharp --input 101-pipeline-3node-transform-chain/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("TransformChainPipeline", 3090);
p.Sink("transform_gateway", 3090, "/transform");
p.Source("chained_transform_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.Compile();
