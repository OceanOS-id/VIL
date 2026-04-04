// 003-basic-hello-server — Zig SDK equivalent
// Compile: vil compile --from zig --input 003-basic-hello-server/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("vil-basic-hello-server", 8080);
    var gw = vil.Service.init("gw");
    gw.endpoint("POST", "/transform", "transform");
    gw.endpoint("POST", "/echo", "echo");
    gw.endpoint("GET", "/health", "health");
    server.service(&gw);
    server.compile();
}
