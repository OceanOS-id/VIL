// 202-llm-multi-model-routing — Zig SDK equivalent
// Compile: vil compile --from zig --input 202-llm-multi-model-routing/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("MultiModelPipeline_GPT4", 8080);
    server.compile();
}
