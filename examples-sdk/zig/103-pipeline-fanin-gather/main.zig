// 103-pipeline-fanin-gather — Zig SDK equivalent
// Compile: vil compile --from zig --input 103-pipeline-fanin-gather/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var p = vil.Pipeline.init("CreditGatherPipeline", 3093);
    p.sink("credit_gather_sink", 3093, "/gather");
    p.source("credit_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
    p.sink("inventory_gather_sink", 3094, "/inventory");
    p.source("inventory_source", "http://localhost:18092/api/v1/products");
    p.route("credit_sink.trigger_out", "credit_source.trigger_in", "LoanWrite");
    p.route("credit_source.response_data_out", "credit_sink.response_data_in", "LoanWrite");
    p.route("credit_source.response_ctrl_out", "credit_sink.response_ctrl_in", "Copy");
    p.route("inventory_sink.trigger_out", "inventory_source.trigger_in", "LoanWrite");
    p.route("inventory_source.response_data_out", "inventory_sink.response_data_in", "LoanWrite");
    p.route("inventory_source.response_ctrl_out", "inventory_sink.response_ctrl_in", "Copy");
    p.compile();
}
