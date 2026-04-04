// 505-villog-tracing-bridge — Zig SDK equivalent
// Compile: vil compile --from zig --input 505-villog-tracing-bridge/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
