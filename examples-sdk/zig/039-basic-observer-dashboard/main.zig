// 039-basic-observer-dashboard — Zig SDK equivalent
// Compile: vil compile --from zig --input 039-basic-observer-dashboard/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("observer-demo", 8080);
    var demo = vil.Service.init("demo");
    demo.endpoint("GET", "/hello", "hello");
    demo.endpoint("POST", "/echo", "echo");
    server.service(&demo);
    server.compile();
}
