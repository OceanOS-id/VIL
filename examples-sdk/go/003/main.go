// 003 — Hello Server (VX_APP)
// Equivalent to: examples/003-basic-hello-server (Rust)
// Compile: vil compile --from go --input 003/main.go --release
package main

import (
	"fmt"
	"os"

	vil "github.com/OceanOS-id/vil-go"
)

func main() {
	server := vil.NewServer("hello-server", 8080)

	// -- ServiceProcess: hello (prefix: /api/hello) ---------------------------
	hello := vil.NewServiceProcess("hello")
	hello.Endpoint("GET", "/", "hello")
	hello.Endpoint("GET", "/greet/:name", "greet")
	hello.Endpoint("POST", "/echo", "echo")
	hello.Endpoint("GET", "/shm-info", "shm_info")
	server.Service(hello, "/api/hello")

	// Built-in: GET /health, /ready, /metrics, /info

	// -- Emit / compile -------------------------------------------------------
	if os.Getenv("VIL_COMPILE_MODE") == "manifest" {
		fmt.Println(server.ToYAML())
	} else {
		server.Compile()
	}
}
