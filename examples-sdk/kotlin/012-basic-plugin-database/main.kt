// 012-basic-plugin-database — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 012-basic-plugin-database/main.kt --release

fun main() {
    val server = VilServer("plugin-database", 8080)
    val plugin_db = ServiceProcess("plugin-db")
    plugin_db.endpoint("GET", "/", "index")
    plugin_db.endpoint("GET", "/plugins", "list_plugins")
    plugin_db.endpoint("GET", "/config", "show_config")
    plugin_db.endpoint("GET", "/products", "list_products")
    plugin_db.endpoint("POST", "/tasks", "create_task")
    plugin_db.endpoint("GET", "/tasks", "list_tasks")
    plugin_db.endpoint("GET", "/pool-stats", "pool_stats")
    plugin_db.endpoint("GET", "/redis-ping", "redis_ping")
    server.service(plugin_db)
    server.compile()
}
