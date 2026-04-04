// 507-villog-bench-file-drain — Zig SDK equivalent
// Compile: vil compile --from zig --input 507-villog-bench-file-drain/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
