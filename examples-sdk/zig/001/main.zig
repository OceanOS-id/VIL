// 001 — AI Gateway (SSE Pipeline)
// Equivalent to: examples/001-basic-ai-gw-demo (Rust)
// Compile: vil compile --from zig --input 001/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var pipeline = vil.VilPipeline.init("ai-gateway", 3080);

    // Nodes
    pipeline.sink("webhook", 3080, "/trigger");
    pipeline.source("inference", "http://localhost:4545/v1/chat/completions", .{
        .format = "sse",
        .dialect = "openai",
        .json_tap = "choices[0].delta.content",
    });

    // Tri-Lane routes
    pipeline.route("webhook.trigger_out", "inference.trigger_in", "LoanWrite");
    pipeline.route("inference.data_out", "webhook.data_in", "LoanWrite");
    pipeline.route("inference.ctrl_out", "webhook.ctrl_in", "Copy");

    pipeline.compile();
}
