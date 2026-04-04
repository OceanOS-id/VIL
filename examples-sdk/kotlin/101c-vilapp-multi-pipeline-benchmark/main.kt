// 101c-vilapp-multi-pipeline-benchmark — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 101c-vilapp-multi-pipeline-benchmark/main.kt --release

fun main() {
    val server = VilServer("multi-pipeline-bench", 3090)
    val pipeline = ServiceProcess("pipeline")
    server.service(pipeline)
    server.compile()
}
