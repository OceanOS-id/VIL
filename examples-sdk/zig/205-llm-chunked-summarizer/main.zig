// 205-llm-chunked-summarizer — Zig SDK equivalent
// Compile: vil compile --from zig --input 205-llm-chunked-summarizer/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("ChunkedSummarizerPipeline", 8080);
    server.compile();
}
