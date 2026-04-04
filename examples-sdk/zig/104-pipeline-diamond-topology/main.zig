// 104-pipeline-diamond-topology — Zig SDK equivalent
// Compile: vil compile --from zig --input 104-pipeline-diamond-topology/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var p = vil.Pipeline.init("DiamondSummary", 3095);
    p.sink("summary_sink", 3095, "/diamond");
    p.source("summary_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
    p.sink("detail_sink", 3096, "/diamond-detail");
    p.source("detail_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
    p.route("summary_sink.trigger_out", "summary_source.trigger_in", "LoanWrite");
    p.route("summary_source.response_data_out", "summary_sink.response_data_in", "LoanWrite");
    p.route("summary_source.response_ctrl_out", "summary_sink.response_ctrl_in", "Copy");
    p.route("detail_sink.trigger_out", "detail_source.trigger_in", "LoanWrite");
    p.route("detail_source.response_data_out", "detail_sink.response_data_in", "LoanWrite");
    p.route("detail_source.response_ctrl_out", "detail_sink.response_ctrl_in", "Copy");
    p.compile();
}
