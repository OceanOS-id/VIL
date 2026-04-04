// 701-mq-rabbitmq-pubsub — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 701-mq-rabbitmq-pubsub/main.kt --release

fun main() {
    val server = VilServer("app", 8080)
    server.compile()
}
