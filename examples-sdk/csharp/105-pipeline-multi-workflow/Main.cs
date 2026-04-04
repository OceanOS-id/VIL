// 105-pipeline-multi-workflow — C# SDK equivalent
// Compile: vil compile --from csharp --input 105-pipeline-multi-workflow/Main.cs --release

#r "sdk/csharp/Vil.cs"

var p = new VilPipeline("AiGatewayWorkflow", 3097);
p.Sink("ai_gateway_sink", 3097, "/ai");
p.Source("ai_sse_source", "http://127.0.0.1:4545/v1/chat/completions", "sse");
p.Sink("credit_sink", 3098, "/credit");
p.Source("credit_ndjson_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
p.Sink("inventory_sink", 3099, "/inventory");
p.Source("inventory_rest_source", "http://localhost:18092/api/v1/products");
p.Route("ai_sink.trigger_out", "ai_source.trigger_in", "LoanWrite");
p.Route("ai_source.response_data_out", "ai_sink.response_data_in", "LoanWrite");
p.Route("ai_source.response_ctrl_out", "ai_sink.response_ctrl_in", "Copy");
p.Route("credit_sink.trigger_out", "credit_source.trigger_in", "LoanWrite");
p.Route("credit_source.response_data_out", "credit_sink.response_data_in", "LoanWrite");
p.Route("credit_source.response_ctrl_out", "credit_sink.response_ctrl_in", "Copy");
p.Route("inventory_sink.trigger_out", "inventory_source.trigger_in", "LoanWrite");
p.Route("inventory_source.response_data_out", "inventory_sink.response_data_in", "LoanWrite");
p.Route("inventory_source.response_ctrl_out", "inventory_sink.response_ctrl_in", "Copy");
p.Compile();
