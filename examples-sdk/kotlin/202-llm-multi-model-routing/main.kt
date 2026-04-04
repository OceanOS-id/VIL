// 202-llm-multi-model-routing — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 202-llm-multi-model-routing/main.kt --release

fun main() {
    val server = VilServer("MultiModelPipeline_GPT4", 8080)
    server.compile()
}
