// 030-basic-trilane-messaging — Go SDK equivalent
// Compile: vil compile --from go --input 030-basic-trilane-messaging/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("ecommerce-order-pipeline", 8080)

	gateway := vil.NewService("gateway")
	s.Service(gateway)

	fulfillment := vil.NewService("fulfillment")
	fulfillment.Endpoint("GET", "/status", "fulfillment_status")
	s.Service(fulfillment)

	s.Compile()
}
