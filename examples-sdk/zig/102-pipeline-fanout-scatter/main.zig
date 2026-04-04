// 102-pipeline-fanout-scatter — Zig SDK equivalent
// Compile: vil compile --from zig --input 102-pipeline-fanout-scatter/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var p = vil.Pipeline.init("NplPipeline", 3091);
    p.sink("npl_sink", 3091, "/npl");
    p.source("npl_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
    p.sink("healthy_sink", 3092, "/healthy");
    p.source("healthy_source", "http://localhost:18081/api/v1/credits/ndjson?count=100", "json");
    p.route("npl_sink.trigger_out", "npl_source.trigger_in", "LoanWrite");
    p.route("npl_source.response_data_out", "npl_sink.response_data_in", "LoanWrite");
    p.route("npl_source.response_ctrl_out", "npl_sink.response_ctrl_in", "Copy");
    p.route("healthy_sink.trigger_out", "healthy_source.trigger_in", "LoanWrite");
    p.route("healthy_source.response_data_out", "healthy_sink.response_data_in", "LoanWrite");
    p.route("healthy_source.response_ctrl_out", "healthy_sink.response_ctrl_in", "Copy");
    p.compile();
}
