// 406-agent-vil-handler-shm — Zig SDK equivalent
// Compile: vil compile --from zig --input 406-agent-vil-handler-shm/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("fraud-detection-agent", 3126);
    var fraud_agent = vil.Service.init("fraud-agent");
    fraud_agent.endpoint("POST", "/detect", "detect_fraud");
    fraud_agent.endpoint("GET", "/health", "health");
    server.service(&fraud_agent);
    server.compile();
}
