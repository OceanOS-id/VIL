// 003-basic-hello-server — Go SDK equivalent
// Compile: vil compile --from go --input 003-basic-hello-server/main.go --release
//
// Minimal REST API: JSON transform, echo, health check.
// Switch mode: VIL_MODE=sidecar (default) / VIL_MODE=wasm

package main

import (
	"example/003-basic-hello-server/handlers"

	vil "github.com/OceanOS-id/vil-go"
)

var mode = vil.ModeFromEnv()

var Transform = vil.Handler("HandleTransform", mode, "shm", handlers.HandleTransform)
var Echo = vil.Handler("HandleEcho", mode, "shm", handlers.HandleEcho)
var Health = vil.Handler("HandleHealth", mode, "shm", handlers.HandleHealth)

func main() {
	vil.Run(Transform, Echo, Health)

	s := vil.NewServer("vil-basic-hello-server", 8080)
	gw := vil.NewService("gw")

	gw.Endpoint("POST", "/transform", Transform)
	gw.Endpoint("POST", "/echo", Echo)
	gw.Endpoint("GET", "/health", Health)

	s.Service(gw)
	s.Compile()
}
