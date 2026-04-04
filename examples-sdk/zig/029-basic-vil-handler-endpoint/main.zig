// 029-basic-vil-handler-endpoint — Zig SDK equivalent
// Compile: vil compile --from zig --input 029-basic-vil-handler-endpoint/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("macro-demo", 8080);
    var demo = vil.Service.init("demo");
    demo.endpoint("GET", "/plain", "plain_handler");
    demo.endpoint("GET", "/handled", "handled_handler");
    demo.endpoint("POST", "/endpoint", "endpoint_handler");
    server.service(&demo);
    server.compile();
}
