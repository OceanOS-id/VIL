// 003-basic-hello-server — Go SDK equivalent
// Compile: vil compile --from go --input 003-basic-hello-server/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("vil-basic-hello-server", 8080)

	gw := vil.NewService("gw")
	gw.Endpoint("POST", "/transform", "transform")
	gw.Endpoint("POST", "/echo", "echo")
	gw.Endpoint("GET", "/health", "health")
	s.Service(gw)

	s.Compile()
}
