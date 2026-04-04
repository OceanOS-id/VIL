// 039-basic-observer-dashboard — Go SDK equivalent
// Compile: vil compile --from go --input 039-basic-observer-dashboard/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("observer-demo", 8080)

	demo := vil.NewService("demo")
	demo.Endpoint("GET", "/hello", "hello")
	demo.Endpoint("POST", "/echo", "echo")
	s.Service(demo)

	s.Compile()
}
