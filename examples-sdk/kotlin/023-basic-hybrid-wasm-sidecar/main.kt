// 023-basic-hybrid-wasm-sidecar — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 023-basic-hybrid-wasm-sidecar/main.kt --release

fun main() {
    val server = VilServer("hybrid-pipeline", 8080)
    val pipeline = ServiceProcess("pipeline")
    pipeline.endpoint("GET", "/", "index")
    pipeline.endpoint("POST", "/validate", "validate_order")
    pipeline.endpoint("POST", "/price", "calculate_price")
    pipeline.endpoint("POST", "/fraud", "fraud_check")
    pipeline.endpoint("POST", "/order", "process_order")
    server.service(pipeline)
    server.compile()
}
