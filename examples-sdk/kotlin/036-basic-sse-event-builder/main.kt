// 036-basic-sse-event-builder — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 036-basic-sse-event-builder/main.kt --release

fun main() {
    val server = VilServer("stock-market-ticker", 8080)
    val ticker = ServiceProcess("ticker")
    ticker.endpoint("GET", "/stream", "ticker_stream")
    ticker.endpoint("GET", "/info", "ticker_info")
    server.service(ticker)
    server.compile()
}
