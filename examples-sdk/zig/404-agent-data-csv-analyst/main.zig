// 404-agent-data-csv-analyst — Zig SDK equivalent
// Compile: vil compile --from zig --input 404-agent-data-csv-analyst/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("csv-analyst-agent", 3123);
    var csv_analyst_agent = vil.Service.init("csv-analyst-agent");
    csv_analyst_agent.endpoint("POST", "/csv-analyze", "csv_analyze_handler");
    server.service(&csv_analyst_agent);
    server.compile();
}
