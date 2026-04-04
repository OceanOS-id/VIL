// 401-agent-calculator — Zig SDK equivalent
// Compile: vil compile --from zig --input 401-agent-calculator/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("calculator-agent", 3120);
    var calc_agent = vil.Service.init("calc-agent");
    calc_agent.endpoint("POST", "/calc", "calc_handler");
    server.service(&calc_agent);
    server.compile();
}
