// 032-basic-failover-ha — Go SDK equivalent
// Compile: vil compile --from go --input 032-basic-failover-ha/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("payment-gateway-ha", 8080)

	primary := vil.NewService("primary")
	primary.Endpoint("GET", "/health", "primary_health")
	primary.Endpoint("POST", "/charge", "primary_charge")
	s.Service(primary)

	backup := vil.NewService("backup")
	backup.Endpoint("GET", "/health", "backup_health")
	backup.Endpoint("POST", "/charge", "backup_charge")
	s.Service(backup)

	s.Compile()
}
