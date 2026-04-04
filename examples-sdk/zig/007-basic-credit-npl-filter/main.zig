// 007-basic-credit-npl-filter — Zig SDK equivalent
// Compile: vil compile --from zig --input 007-basic-credit-npl-filter/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var p = vil.Pipeline.init("NplFilterPipeline", 3081);
    p.sink("npl_filter_sink", 3081, "/filter-npl");
    p.source("npl_credit_source", "json");
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
    p.compile();
}
