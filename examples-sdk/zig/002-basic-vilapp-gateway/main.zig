// 002-basic-vilapp-gateway — Zig SDK equivalent
// Compile: vil compile --from zig --input 002-basic-vilapp-gateway/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("vil-app-gateway", 3081);
    var gw = vil.Service.init("gw");
    gw.endpoint("POST", "/trigger", "trigger_handler");
    server.service(&gw);
    server.compile();
}
