// 107-pipeline-process-traced — C# SDK equivalent
// Compile: vil compile --from csharp --input 107-pipeline-process-traced/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("SupplyChainTrackedPipeline", 3107);
p.Sink("tracking_sink", 3107, "/traced");
p.Source("supply_chain_source", "http://localhost:18081/api/v1/credits/stream", "sse");
p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.Route("source.tracking_data_out", "sink.tracking_data_in", "LoanWrite");
p.Route("source.delivery_ctrl_out", "sink.delivery_ctrl_in", "Copy");
p.Compile();
