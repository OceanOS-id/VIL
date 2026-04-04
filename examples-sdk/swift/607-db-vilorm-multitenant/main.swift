// 607-db-vilorm-multitenant — Swift SDK equivalent
// Compile: vil compile --from swift --input 607-db-vilorm-multitenant/main.swift --release

let server = VilServer(name: "vilorm-multitenant", port: 8087)
let saas = ServiceProcess(name: "saas")
saas.endpoint(method: "POST", path: "/tenants", handler: "create_tenant")
saas.endpoint(method: "GET", path: "/tenants/:id", handler: "get_tenant")
saas.endpoint(method: "PUT", path: "/tenants/:id", handler: "update_tenant")
saas.endpoint(method: "POST", path: "/tenants/:id/users", handler: "add_user")
saas.endpoint(method: "GET", path: "/tenants/:id/users", handler: "list_users")
saas.endpoint(method: "POST", path: "/tenants/:id/settings", handler: "upsert_setting")
saas.endpoint(method: "GET", path: "/tenants/:id/settings", handler: "list_settings")
saas.endpoint(method: "GET", path: "/tenants/:id/stats", handler: "tenant_stats")
server.service(saas)
server.compile()
