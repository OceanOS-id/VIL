// 009-basic-credit-regulatory-slik — C# SDK equivalent
// Compile: vil compile --from csharp --input 009-basic-credit-regulatory-slik/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("RegulatoryStreamPipeline", 3083);
p.Sink("regulatory_sink", 3083, "/regulatory-stream");
p.Source("regulatory_source", "http://localhost:18081/api/v1/credits/ndjson?count=1000", "json");
p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.Compile();
