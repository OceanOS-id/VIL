// 502-villog-file-rolling — Zig SDK equivalent
// Compile: vil compile --from zig --input 502-villog-file-rolling/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
