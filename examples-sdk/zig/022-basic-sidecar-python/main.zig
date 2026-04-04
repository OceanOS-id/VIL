// 022-basic-sidecar-python — Zig SDK equivalent
// Compile: vil compile --from zig --input 022-basic-sidecar-python/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("sidecar-python-example", 8080);
    var fraud = vil.Service.init("fraud");
    fraud.endpoint("GET", "/status", "fraud_status");
    fraud.endpoint("POST", "/check", "fraud_check");
    server.service(&fraud);
    var root = vil.Service.init("root");
    root.endpoint("GET", "/", "index");
    server.service(&root);
    server.compile();
}
