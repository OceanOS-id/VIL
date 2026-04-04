// 001b-vilapp-ai-gw-benchmark — Zig SDK equivalent
// Compile: vil compile --from zig --input 001b-vilapp-ai-gw-benchmark/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("ai-gw-bench", 3081);
    var gw = vil.Service.init("gw");
    server.service(&gw);
    server.compile();
}
