// 018-basic-ai-multi-model-router — Zig SDK equivalent
// Compile: vil compile --from zig --input 018-basic-ai-multi-model-router/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("ai-multi-model-router", 3085);
    var router = vil.Service.init("router");
    router.endpoint("POST", "/route", "route_handler");
    server.service(&router);
    server.compile();
}
