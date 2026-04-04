// 803-trigger-webhook-receiver — Zig SDK equivalent
// Compile: vil compile --from zig --input 803-trigger-webhook-receiver/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
