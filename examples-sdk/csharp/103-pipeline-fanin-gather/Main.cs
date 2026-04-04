// 103-pipeline-fanin-gather — C# SDK equivalent
// Compile: vil compile --from csharp --input 103-pipeline-fanin-gather/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("CreditGatherPipeline", 3093);
p.Sink("credit_gather_sink", 3093, "/gather");
p.Source("credit_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
p.Sink("inventory_gather_sink", 3094, "/inventory");
p.Source("inventory_source", "http://localhost:18092/api/v1/products");
p.Route("credit_sink.trigger_out", "credit_source.trigger_in", "LoanWrite");
p.Route("credit_source.response_data_out", "credit_sink.response_data_in", "LoanWrite");
p.Route("credit_source.response_ctrl_out", "credit_sink.response_ctrl_in", "Copy");
p.Route("inventory_sink.trigger_out", "inventory_source.trigger_in", "LoanWrite");
p.Route("inventory_source.response_data_out", "inventory_sink.response_data_in", "LoanWrite");
p.Route("inventory_source.response_ctrl_out", "inventory_sink.response_ctrl_in", "Copy");
p.Compile();
