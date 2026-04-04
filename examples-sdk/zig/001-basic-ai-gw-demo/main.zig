// 001-basic-ai-gw-demo — Zig SDK equivalent
// Compile: vil compile --from zig --input 001-basic-ai-gw-demo/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var p = vil.Pipeline.init("DecomposedPipeline", 3080);
    p.sink("webhook_trigger", 3080, "/trigger");
    p.source("sse_inference", "http://127.0.0.1:4545/v1/chat/completions", "sse");
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
    p.compile();
}
