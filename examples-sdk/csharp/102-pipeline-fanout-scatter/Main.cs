// 102-pipeline-fanout-scatter — C# SDK equivalent
// Compile: vil compile --from csharp --input 102-pipeline-fanout-scatter/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("NplPipeline", 3091);
p.Sink("npl_sink", 3091, "/npl");
p.Source("npl_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
p.Sink("healthy_sink", 3092, "/healthy");
p.Source("healthy_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
p.Route("npl_sink.trigger_out", "npl_source.trigger_in", "LoanWrite");
p.Route("npl_source.response_data_out", "npl_sink.response_data_in", "LoanWrite");
p.Route("npl_source.response_ctrl_out", "npl_sink.response_ctrl_in", "Copy");
p.Route("healthy_sink.trigger_out", "healthy_source.trigger_in", "LoanWrite");
p.Route("healthy_source.response_data_out", "healthy_sink.response_data_in", "LoanWrite");
p.Route("healthy_source.response_ctrl_out", "healthy_sink.response_ctrl_in", "Copy");
p.Compile();
