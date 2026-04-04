// 018-basic-ai-multi-model-router — Swift SDK equivalent
// Compile: vil compile --from swift --input 018-basic-ai-multi-model-router/main.swift --release

let server = VilServer(name: "ai-multi-model-router", port: 3085)
let router = ServiceProcess(name: "router")
router.endpoint(method: "POST", path: "/route", handler: "route_handler")
server.service(router)
server.compile()
