// 105-pipeline-multi-workflow — Zig SDK equivalent
// Compile: vil compile --from zig --input 105-pipeline-multi-workflow/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var p = vil.Pipeline.init("AiGatewayWorkflow", 3097);
    p.sink("ai_gateway_sink", 3097, "/ai");
    p.source("ai_sse_source", "http://127.0.0.1:4545/v1/chat/completions", "sse");
    p.sink("credit_sink", 3098, "/credit");
    p.source("credit_ndjson_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
    p.sink("inventory_sink", 3099, "/inventory");
    p.source("inventory_rest_source", "http://localhost:18092/api/v1/products");
    p.route("ai_sink.trigger_out", "ai_source.trigger_in", "LoanWrite");
    p.route("ai_source.response_data_out", "ai_sink.response_data_in", "LoanWrite");
    p.route("ai_source.response_ctrl_out", "ai_sink.response_ctrl_in", "Copy");
    p.route("credit_sink.trigger_out", "credit_source.trigger_in", "LoanWrite");
    p.route("credit_source.response_data_out", "credit_sink.response_data_in", "LoanWrite");
    p.route("credit_source.response_ctrl_out", "credit_sink.response_ctrl_in", "Copy");
    p.route("inventory_sink.trigger_out", "inventory_source.trigger_in", "LoanWrite");
    p.route("inventory_source.response_data_out", "inventory_sink.response_data_in", "LoanWrite");
    p.route("inventory_source.response_ctrl_out", "inventory_sink.response_ctrl_in", "Copy");
    p.compile();
}
