// 027 — VilServer Minimal (No VX)
// Equivalent to: examples/027-basic-vilserver-minimal (Rust)
// Compile: vil compile --from go --input 027/main.go --release
package main

import (
	"fmt"
	"os"

	vil "github.com/OceanOS-id/vil-go"
)

func main() {
	server := vil.NewServer("minimal-api", 8080)

	// -- Fault type -----------------------------------------------------------
	server.Fault("ApiFault", []string{"InvalidInput", "NotFound"})

	// -- Routes (no ServiceProcess, no VX) ------------------------------------
	server.Route("GET", "/hello", "hello")
	server.Route("POST", "/echo", "echo")

	// Built-in: GET /health, /ready, /metrics, /info

	// -- Emit / compile -------------------------------------------------------
	if os.Getenv("VIL_COMPILE_MODE") == "manifest" {
		fmt.Println(server.ToYAML())
	} else {
		server.Compile()
	}
}
