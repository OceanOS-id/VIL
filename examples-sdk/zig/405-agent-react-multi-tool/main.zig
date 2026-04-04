// 405-agent-react-multi-tool — Zig SDK equivalent
// Compile: vil compile --from zig --input 405-agent-react-multi-tool/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("react-multi-tool-agent", 3124);
    var react_agent = vil.Service.init("react-agent");
    react_agent.endpoint("POST", "/react", "react_handler");
    server.service(&react_agent);
    server.compile();
}
