// 206-llm-decision-routing — Zig SDK equivalent
// Compile: vil compile --from zig --input 206-llm-decision-routing/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("insurance-underwriting-ai", 3116);
    var underwriter = vil.Service.init("underwriter");
    server.service(&underwriter);
    server.compile();
}
