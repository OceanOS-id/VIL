// 601-storage-s3-basic — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 601-storage-s3-basic/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
