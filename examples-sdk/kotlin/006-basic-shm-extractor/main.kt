// 006-basic-shm-extractor — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 006-basic-shm-extractor/main.kt --release

fun main() {
    val server = VilServer("shm-extractor-demo", 8080)
    val shm_demo = ServiceProcess("shm-demo")
    shm_demo.endpoint("POST", "/ingest", "ingest")
    shm_demo.endpoint("POST", "/compute", "compute")
    shm_demo.endpoint("GET", "/shm-stats", "shm_stats")
    shm_demo.endpoint("GET", "/benchmark", "benchmark")
    server.service(shm_demo)
    server.compile()
}
