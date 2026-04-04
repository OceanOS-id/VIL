// 804-trigger-cdc-postgres — Zig SDK equivalent
// Compile: vil compile --from zig --input 804-trigger-cdc-postgres/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("app", 8080);
    server.compile();
}
