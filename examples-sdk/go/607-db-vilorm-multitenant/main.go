// 607-db-vilorm-multitenant — Go SDK equivalent
// Compile: vil compile --from go --input 607-db-vilorm-multitenant/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("vilorm-multitenant", 8087)

	saas := vil.NewService("saas")
	saas.Endpoint("POST", "/tenants", "create_tenant")
	saas.Endpoint("GET", "/tenants/:id", "get_tenant")
	saas.Endpoint("PUT", "/tenants/:id", "update_tenant")
	saas.Endpoint("POST", "/tenants/:id/users", "add_user")
	saas.Endpoint("GET", "/tenants/:id/users", "list_users")
	saas.Endpoint("POST", "/tenants/:id/settings", "upsert_setting")
	saas.Endpoint("GET", "/tenants/:id/settings", "list_settings")
	saas.Endpoint("GET", "/tenants/:id/stats", "tenant_stats")
	s.Service(saas)

	s.Compile()
}
