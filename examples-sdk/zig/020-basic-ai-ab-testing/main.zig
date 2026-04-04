// 020-basic-ai-ab-testing — Zig SDK equivalent
// Compile: vil compile --from zig --input 020-basic-ai-ab-testing/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("ai-ab-testing-gateway", 8080);
    var ab = vil.Service.init("ab");
    ab.endpoint("POST", "/infer", "infer");
    ab.endpoint("GET", "/metrics", "metrics");
    ab.endpoint("POST", "/config", "update_config");
    server.service(&ab);
    var root = vil.Service.init("root");
    root.endpoint("GET", "/", "index");
    server.service(&root);
    server.compile();
}
