// 204-llm-streaming-translator — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 204-llm-streaming-translator/main.kt --release

fun main() {
    val server = VilServer("llm-streaming-translator", 3103)
    val translator = ServiceProcess("translator")
    server.service(translator)
    server.compile()
}
