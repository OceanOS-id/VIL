// 035-basic-vil-service-module — Zig SDK equivalent
// Compile: vil compile --from zig --input 035-basic-vil-service-module/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
