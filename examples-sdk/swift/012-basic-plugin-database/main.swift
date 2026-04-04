// 012-basic-plugin-database — Swift SDK equivalent
// Compile: vil compile --from swift --input 012-basic-plugin-database/main.swift --release

let server = VilServer(name: "plugin-database", port: 8080)
let plugin_db = ServiceProcess(name: "plugin-db")
plugin_db.endpoint(method: "GET", path: "/", handler: "index")
plugin_db.endpoint(method: "GET", path: "/plugins", handler: "list_plugins")
plugin_db.endpoint(method: "GET", path: "/config", handler: "show_config")
plugin_db.endpoint(method: "GET", path: "/products", handler: "list_products")
plugin_db.endpoint(method: "POST", path: "/tasks", handler: "create_task")
plugin_db.endpoint(method: "GET", path: "/tasks", handler: "list_tasks")
plugin_db.endpoint(method: "GET", path: "/pool-stats", handler: "pool_stats")
plugin_db.endpoint(method: "GET", path: "/redis-ping", handler: "redis_ping")
server.service(plugin_db)
server.compile()
