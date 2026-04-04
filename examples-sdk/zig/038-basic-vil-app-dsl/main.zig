// 038-basic-vil-app-dsl — Zig SDK equivalent
// Compile: vil compile --from zig --input 038-basic-vil-app-dsl/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
