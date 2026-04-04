// 033-basic-shm-write-through — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 033-basic-shm-write-through/main.kt --release

fun main() {
    val server = VilServer("realtime-analytics-dashboard", 8080)
    val catalog = ServiceProcess("catalog")
    catalog.endpoint("POST", "/catalog/search", "catalog_search")
    catalog.endpoint("GET", "/catalog/health", "catalog_health")
    server.service(catalog)
    server.compile()
}
