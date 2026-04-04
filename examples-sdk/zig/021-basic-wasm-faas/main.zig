// 021-basic-wasm-faas — Zig SDK equivalent
// Compile: vil compile --from zig --input 021-basic-wasm-faas/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("wasm-faas-example", 8080);
    var wasm_faas = vil.Service.init("wasm-faas");
    wasm_faas.endpoint("GET", "/", "index");
    wasm_faas.endpoint("GET", "/wasm/modules", "list_modules");
    wasm_faas.endpoint("POST", "/wasm/pricing", "invoke_pricing");
    wasm_faas.endpoint("POST", "/wasm/validation", "invoke_validation");
    wasm_faas.endpoint("POST", "/wasm/transform", "invoke_transform");
    server.service(&wasm_faas);
    server.compile();
}
