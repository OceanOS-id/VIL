// 101b-multi-pipeline-benchmark — Zig SDK equivalent
// Compile: vil compile --from zig --input 101b-multi-pipeline-benchmark/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var p = vil.Pipeline.init("MultiPipelineBench", 3090);
    p.sink("gateway", 3090, "/trigger");
    p.source("l_l_m_upstream", "http://127.0.0.1:4545/v1/chat/completions");
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
    p.compile();
}
