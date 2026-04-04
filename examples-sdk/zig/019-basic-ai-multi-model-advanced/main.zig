// 019-basic-ai-multi-model-advanced — Zig SDK equivalent
// Compile: vil compile --from zig --input 019-basic-ai-multi-model-advanced/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var p = vil.Pipeline.init("AdvancedMultiModelRouterPipeline", 3086);
    p.sink("advanced_router_sink", 3086, "/route-advanced");
    p.source("advanced_router_source", "http://127.0.0.1:4545/v1/chat/completions", "sse");
    p.route("sink.trigger_out", "source.trigger_in", "LoanWrite");
    p.route("source.response_data_out", "sink.response_data_in", "LoanWrite");
    p.route("source.response_ctrl_out", "sink.response_ctrl_in", "Copy");
    p.compile();
}
