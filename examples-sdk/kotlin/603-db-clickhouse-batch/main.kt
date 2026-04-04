// 603-db-clickhouse-batch — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 603-db-clickhouse-batch/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
