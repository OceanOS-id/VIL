// 008-basic-credit-quality-monitor — Zig SDK equivalent
// Compile: vil compile --from zig --input 008-basic-credit-quality-monitor/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var p = vil.Pipeline.init("CreditQualityMonitorPipeline", 3082);
    p.sink("quality_monitor_sink", 3082, "/quality-check");
    p.source("quality_credit_source", "json");
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
    p.compile();
}
