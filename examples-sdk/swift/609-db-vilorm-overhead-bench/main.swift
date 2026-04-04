// 609-db-vilorm-overhead-bench — Swift SDK equivalent
// Compile: vil compile --from swift --input 609-db-vilorm-overhead-bench/main.swift --release

let server = VilServer(name: "overhead-bench", port: 8099)
let bench = ServiceProcess(name: "bench")
bench.endpoint(method: "GET", path: "/raw/items/:id", handler: "raw_find_by_id")
bench.endpoint(method: "GET", path: "/raw/items", handler: "raw_list")
bench.endpoint(method: "GET", path: "/raw/count", handler: "raw_count")
bench.endpoint(method: "GET", path: "/raw/cols", handler: "raw_select_cols")
bench.endpoint(method: "GET", path: "/orm/items/:id", handler: "orm_find_by_id")
bench.endpoint(method: "GET", path: "/orm/items", handler: "orm_list")
bench.endpoint(method: "GET", path: "/orm/count", handler: "orm_count")
bench.endpoint(method: "GET", path: "/orm/cols", handler: "orm_select_cols")
server.service(bench)
server.compile()
