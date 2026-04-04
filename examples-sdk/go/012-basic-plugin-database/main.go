// 012-basic-plugin-database — Go SDK equivalent
// Compile: vil compile --from go --input 012-basic-plugin-database/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("plugin-database", 8080)

	plugin_db := vil.NewService("plugin-db")
	plugin_db.Endpoint("GET", "/", "index")
	plugin_db.Endpoint("GET", "/plugins", "list_plugins")
	plugin_db.Endpoint("GET", "/config", "show_config")
	plugin_db.Endpoint("GET", "/products", "list_products")
	plugin_db.Endpoint("POST", "/tasks", "create_task")
	plugin_db.Endpoint("GET", "/tasks", "list_tasks")
	plugin_db.Endpoint("GET", "/pool-stats", "pool_stats")
	plugin_db.Endpoint("GET", "/redis-ping", "redis_ping")
	s.Service(plugin_db)

	s.Compile()
}
