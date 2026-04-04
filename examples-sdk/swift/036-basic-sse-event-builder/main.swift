// 036-basic-sse-event-builder — Swift SDK equivalent
// Compile: vil compile --from swift --input 036-basic-sse-event-builder/main.swift --release

let server = VilServer(name: "stock-market-ticker", port: 8080)
let ticker = ServiceProcess(name: "ticker")
ticker.endpoint(method: "GET", path: "/stream", handler: "ticker_stream")
ticker.endpoint(method: "GET", path: "/info", handler: "ticker_info")
server.service(ticker)
server.compile()
