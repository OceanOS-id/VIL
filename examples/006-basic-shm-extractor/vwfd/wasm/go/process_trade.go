// Trade Data Processor — Go WASM Module (TinyGo)
// Build: tinygo build -o process_trade.wasm -target wasi process_trade.go
package main

import (
	"encoding/json"
	"fmt"
	"os"
)

type TradeInput struct {
	Symbol string  `json:"symbol"`
	Price  float64 `json:"price"`
	Volume int64   `json:"volume"`
	Side   string  `json:"side"`
}

type TradeOutput struct {
	Symbol    string  `json:"symbol"`
	Price     float64 `json:"price"`
	Volume    int64   `json:"volume"`
	Side      string  `json:"side"`
	Notional  float64 `json:"notional"`
	Processed bool    `json:"processed"`
}

func main() {
	var input TradeInput
	decoder := json.NewDecoder(os.Stdin)
	if err := decoder.Decode(&input); err != nil {
		fmt.Fprintf(os.Stderr, "parse error: %v\n", err)
		os.Exit(1)
	}

	output := TradeOutput{
		Symbol:    input.Symbol,
		Price:     input.Price,
		Volume:    input.Volume,
		Side:      input.Side,
		Notional:  input.Price * float64(input.Volume),
		Processed: true,
	}

	json.NewEncoder(os.Stdout).Encode(output)
}
