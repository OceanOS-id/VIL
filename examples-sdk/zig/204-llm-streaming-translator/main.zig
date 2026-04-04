// 204-llm-streaming-translator — Zig SDK equivalent
// Compile: vil compile --from zig --input 204-llm-streaming-translator/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("llm-streaming-translator", 3103);
    var translator = vil.Service.init("translator");
    server.service(&translator);
    server.compile();
}
