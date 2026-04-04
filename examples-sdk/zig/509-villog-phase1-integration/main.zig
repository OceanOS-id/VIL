// 509-villog-phase1-integration — Zig SDK equivalent
// Compile: vil compile --from zig --input 509-villog-phase1-integration/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
