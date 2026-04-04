// 021-basic-wasm-faas — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 021-basic-wasm-faas/main.kt --release

fun main() {
    val server = VilServer("wasm-faas-example", 8080)
    val wasm_faas = ServiceProcess("wasm-faas")
    wasm_faas.endpoint("GET", "/", "index")
    wasm_faas.endpoint("GET", "/wasm/modules", "list_modules")
    wasm_faas.endpoint("POST", "/wasm/pricing", "invoke_pricing")
    wasm_faas.endpoint("POST", "/wasm/validation", "invoke_validation")
    wasm_faas.endpoint("POST", "/wasm/transform", "invoke_transform")
    server.service(wasm_faas)
    server.compile()
}
