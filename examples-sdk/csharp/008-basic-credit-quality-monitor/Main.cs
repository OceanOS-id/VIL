// 008-basic-credit-quality-monitor — C# SDK equivalent
// Compile: vil compile --from csharp --input 008-basic-credit-quality-monitor/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("CreditQualityMonitorPipeline", 3082);
p.Sink("quality_monitor_sink", 3082, "/quality-check");
p.Source("quality_credit_source", "json");
p.Route("sink.trigger_out", "source.trigger_in", "LoanWrite");
p.Route("source.response_data_out", "sink.response_data_in", "LoanWrite");
p.Route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
p.Compile();
