// 034-basic-blocking-task — Zig SDK equivalent
// Compile: vil compile --from zig --input 034-basic-blocking-task/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("credit-risk-scoring-engine", 8080);
    var risk_engine = vil.Service.init("risk-engine");
    risk_engine.endpoint("POST", "/risk/assess", "assess_risk");
    risk_engine.endpoint("GET", "/risk/health", "risk_health");
    server.service(&risk_engine);
    server.compile();
}
