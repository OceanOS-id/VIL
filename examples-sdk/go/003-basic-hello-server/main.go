// 003 — Hello Server (VX_APP)
// Equivalent to: examples/003-basic-hello-server (Rust)
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	server := vil.NewServer("vil-basic-hello-server", 8080)

	gw := vil.NewService("gw")
	gw.Endpoint("POST", "/transform", "transform")
	gw.Endpoint("POST", "/echo", "echo")
	gw.Endpoint("GET", "/health", "health")
	server.Service(gw)

	server.Compile()
}
