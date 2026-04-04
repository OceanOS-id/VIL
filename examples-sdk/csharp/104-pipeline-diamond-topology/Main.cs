// 104-pipeline-diamond-topology — C# SDK equivalent
// Compile: vil compile --from csharp --input 104-pipeline-diamond-topology/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("DiamondSummary", 3095);
p.Sink("summary_sink", 3095, "/diamond");
p.Source("summary_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
p.Sink("detail_sink", 3096, "/diamond-detail");
p.Source("detail_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
p.Route("summary_sink.trigger_out", "summary_source.trigger_in", "LoanWrite");
p.Route("summary_source.response_data_out", "summary_sink.response_data_in", "LoanWrite");
p.Route("summary_source.response_ctrl_out", "summary_sink.response_ctrl_in", "Copy");
p.Route("detail_sink.trigger_out", "detail_source.trigger_in", "LoanWrite");
p.Route("detail_source.response_data_out", "detail_sink.response_data_in", "LoanWrite");
p.Route("detail_source.response_ctrl_out", "detail_sink.response_ctrl_in", "Copy");
p.Compile();
