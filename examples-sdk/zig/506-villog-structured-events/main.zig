// 506-villog-structured-events — Zig SDK equivalent
// Compile: vil compile --from zig --input 506-villog-structured-events/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
