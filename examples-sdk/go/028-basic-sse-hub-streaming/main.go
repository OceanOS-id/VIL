// 028-basic-sse-hub-streaming — Go SDK equivalent
// Compile: vil compile --from go --input 028-basic-sse-hub-streaming/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("sse-hub-demo", 8080)

	events := vil.NewService("events")
	events.Endpoint("POST", "/publish", "publish")
	events.Endpoint("GET", "/stream", "stream")
	events.Endpoint("GET", "/stats", "stats")
	s.Service(events)

	s.Compile()
}
