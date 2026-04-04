// 021-basic-wasm-faas — Swift SDK equivalent
// Compile: vil compile --from swift --input 021-basic-wasm-faas/main.swift --release

let server = VilServer(name: "wasm-faas-example", port: 8080)
let wasm_faas = ServiceProcess(name: "wasm-faas")
wasm_faas.endpoint(method: "GET", path: "/", handler: "index")
wasm_faas.endpoint(method: "GET", path: "/wasm/modules", handler: "list_modules")
wasm_faas.endpoint(method: "POST", path: "/wasm/pricing", handler: "invoke_pricing")
wasm_faas.endpoint(method: "POST", path: "/wasm/validation", handler: "invoke_validation")
wasm_faas.endpoint(method: "POST", path: "/wasm/transform", handler: "invoke_transform")
server.service(wasm_faas)
server.compile()
