// 033-basic-shm-write-through — Go SDK equivalent
// Compile: vil compile --from go --input 033-basic-shm-write-through/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("realtime-analytics-dashboard", 8080)

	catalog := vil.NewService("catalog")
	catalog.Endpoint("POST", "/catalog/search", "catalog_search")
	catalog.Endpoint("GET", "/catalog/health", "catalog_health")
	s.Service(catalog)

	s.Compile()
}
