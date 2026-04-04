// 033-basic-shm-write-through — Swift SDK equivalent
// Compile: vil compile --from swift --input 033-basic-shm-write-through/main.swift --release

let server = VilServer(name: "realtime-analytics-dashboard", port: 8080)
let catalog = ServiceProcess(name: "catalog")
catalog.endpoint(method: "POST", path: "/catalog/search", handler: "catalog_search")
catalog.endpoint(method: "GET", path: "/catalog/health", handler: "catalog_health")
server.service(catalog)
server.compile()
