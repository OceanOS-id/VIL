// 609-db-vilorm-overhead-bench — Go SDK equivalent
// Compile: vil compile --from go --input 609-db-vilorm-overhead-bench/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("overhead-bench", 8099)

	bench := vil.NewService("bench")
	bench.Endpoint("GET", "/raw/items/:id", "raw_find_by_id")
	bench.Endpoint("GET", "/raw/items", "raw_list")
	bench.Endpoint("GET", "/raw/count", "raw_count")
	bench.Endpoint("GET", "/raw/cols", "raw_select_cols")
	bench.Endpoint("GET", "/orm/items/:id", "orm_find_by_id")
	bench.Endpoint("GET", "/orm/items", "orm_list")
	bench.Endpoint("GET", "/orm/count", "orm_count")
	bench.Endpoint("GET", "/orm/cols", "orm_select_cols")
	s.Service(bench)

	s.Compile()
}
