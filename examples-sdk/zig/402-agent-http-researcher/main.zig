// 402-agent-http-researcher — Zig SDK equivalent
// Compile: vil compile --from zig --input 402-agent-http-researcher/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("http-researcher-agent", 3121);
    var research_agent = vil.Service.init("research-agent");
    research_agent.endpoint("POST", "/research", "research_handler");
    research_agent.endpoint("GET", "/products", "products_handler");
    server.service(&research_agent);
    server.compile();
}
