// 027 — VilServer Minimal (No VX)
// Equivalent to: examples/027-basic-vilserver-minimal (Rust)
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	server := vil.NewServer("app", 8080)
	server.Compile()
}
