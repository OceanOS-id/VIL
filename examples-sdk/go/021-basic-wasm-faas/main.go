// 021-basic-wasm-faas — Go SDK equivalent
// Compile: vil compile --from go --input 021-basic-wasm-faas/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("wasm-faas-example", 8080)

	wasm_faas := vil.NewService("wasm-faas")
	wasm_faas.Endpoint("GET", "/", "index")
	wasm_faas.Endpoint("GET", "/wasm/modules", "list_modules")
	wasm_faas.Endpoint("POST", "/wasm/pricing", "invoke_pricing")
	wasm_faas.Endpoint("POST", "/wasm/validation", "invoke_validation")
	wasm_faas.Endpoint("POST", "/wasm/transform", "invoke_transform")
	s.Service(wasm_faas)

	s.Compile()
}
