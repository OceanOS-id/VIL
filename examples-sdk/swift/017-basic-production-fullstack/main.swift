// 017-basic-production-fullstack — Swift SDK equivalent
// Compile: vil compile --from swift --input 017-basic-production-fullstack/main.swift --release

let server = VilServer(name: "production-fullstack", port: 8080)
let fullstack = ServiceProcess(name: "fullstack")
fullstack.endpoint(method: "GET", path: "/stack", handler: "stack_info")
fullstack.endpoint(method: "GET", path: "/config", handler: "full_config")
fullstack.endpoint(method: "GET", path: "/sprints", handler: "sprints")
fullstack.endpoint(method: "GET", path: "/middleware", handler: "middleware_info")
server.service(fullstack)
let admin = ServiceProcess(name: "admin")
admin.endpoint(method: "GET", path: "/config", handler: "full_config")
server.service(admin)
let root = ServiceProcess(name: "root")
server.service(root)
server.compile()
