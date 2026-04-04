// 302-rag-multi-source-fanin — Go SDK equivalent
// Compile: vil compile --from go --input 302-rag-multi-source-fanin/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	p := vil.NewPipeline("rag-multi-source-fanin", 3111)

	p.Compile()
}
