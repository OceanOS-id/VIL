// 702-mq-sqs-send-receive — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 702-mq-sqs-send-receive/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
