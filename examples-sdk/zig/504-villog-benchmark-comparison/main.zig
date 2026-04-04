// 504-villog-benchmark-comparison — Zig SDK equivalent
// Compile: vil compile --from zig --input 504-villog-benchmark-comparison/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
