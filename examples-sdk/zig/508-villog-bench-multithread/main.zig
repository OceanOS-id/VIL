// 508-villog-bench-multithread — Zig SDK equivalent
// Compile: vil compile --from zig --input 508-villog-bench-multithread/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
