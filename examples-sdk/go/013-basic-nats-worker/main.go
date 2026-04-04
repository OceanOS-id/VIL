// 013-basic-nats-worker — Go SDK equivalent
// Compile: vil compile --from go --input 013-basic-nats-worker/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("nats-worker", 8080)

	nats := vil.NewService("nats")
	nats.Endpoint("GET", "/nats/config", "nats_config")
	nats.Endpoint("POST", "/nats/publish", "nats_publish")
	nats.Endpoint("GET", "/nats/jetstream", "jetstream_info")
	nats.Endpoint("GET", "/nats/kv", "kv_demo")
	s.Service(nats)

	root := vil.NewService("root")
	s.Service(root)

	s.Compile()
}
