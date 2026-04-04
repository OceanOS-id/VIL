// 029-basic-vil-handler-endpoint — Go SDK equivalent
// Compile: vil compile --from go --input 029-basic-vil-handler-endpoint/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("macro-demo", 8080)

	demo := vil.NewService("demo")
	demo.Endpoint("GET", "/plain", "plain_handler")
	demo.Endpoint("GET", "/handled", "handled_handler")
	demo.Endpoint("POST", "/endpoint", "endpoint_handler")
	s.Service(demo)

	s.Compile()
}
