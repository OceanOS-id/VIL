// 036-basic-sse-event-builder — Go SDK equivalent
// Compile: vil compile --from go --input 036-basic-sse-event-builder/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("stock-market-ticker", 8080)

	ticker := vil.NewService("ticker")
	ticker.Endpoint("GET", "/stream", "ticker_stream")
	ticker.Endpoint("GET", "/info", "ticker_info")
	s.Service(ticker)

	s.Compile()
}
