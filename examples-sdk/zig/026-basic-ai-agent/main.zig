// 026-basic-ai-agent — Zig SDK equivalent
// Compile: vil compile --from zig --input 026-basic-ai-agent/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("ai-agent", 8080);
    var agent = vil.Service.init("agent");
    agent.endpoint("POST", "/agent", "agent_handler");
    server.service(&agent);
    server.compile();
}
