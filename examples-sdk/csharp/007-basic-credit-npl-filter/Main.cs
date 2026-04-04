// 007-basic-credit-npl-filter — C# SDK equivalent
// Compile: vil compile --from csharp --input 007-basic-credit-npl-filter/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("NplFilterPipeline", 3081);
p.Sink("npl_filter_sink", 3081, "/filter-npl");
p.Source("npl_credit_source", "json");
p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.Compile();
