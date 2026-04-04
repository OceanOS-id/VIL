// 023-basic-hybrid-wasm-sidecar — Zig SDK equivalent
// Compile: vil compile --from zig --input 023-basic-hybrid-wasm-sidecar/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("hybrid-pipeline", 8080);
    var pipeline = vil.Service.init("pipeline");
    pipeline.endpoint("GET", "/", "index");
    pipeline.endpoint("POST", "/validate", "validate_order");
    pipeline.endpoint("POST", "/price", "calculate_price");
    pipeline.endpoint("POST", "/fraud", "fraud_check");
    pipeline.endpoint("POST", "/order", "process_order");
    server.service(&pipeline);
    server.compile();
}
