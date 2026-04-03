// =============================================================================
// VIL SDK for Kotlin — Single-file, zero-dependency SDK.
//
// Usage:
//   kotlin main.kt                                  — compiles via `vil compile`
//   VIL_COMPILE_MODE=manifest kotlin main.kt        — prints YAML manifest
//
// API:
//   val p = VilPipeline("name", 3080)
//       .sink("webhook", 3080, "/trigger")
//       .source("inference", "http://...", "sse", jsonTap = "choices[0].delta.content")
//       .route("webhook.trigger_out", "inference.trigger_in", "LoanWrite")
//   p.compile()
// =============================================================================

class ServiceProcess(val name: String, val prefix: String = "/api/$name") {
    internal data class Ep(val method: String, val path: String, val handler: String)
    internal val endpoints = mutableListOf<Ep>()

    fun endpoint(method: String, path: String, handler: String? = null): ServiceProcess {
        val h = handler ?: "${method.lowercase()}_${path.trimStart('/').replace("/", "_").replace(":", "")}"
        endpoints.add(Ep(method, path, h))
        return this
    }
}

class VilPipeline(private val name: String, private var port: Int) {
    private data class Node(val type: String, val port: Int = 0, val path: String? = null,
        val url: String? = null, val format: String? = null,
        val jsonTap: String? = null, val dialect: String? = null)
    private data class Route(val from: String, val to: String, val mode: String)

    private val nodeOrder = mutableListOf<String>()
    private val nodes = mutableMapOf<String, Node>()
    private val routes = mutableListOf<Route>()

    fun sink(nodeName: String, port: Int = 3080, path: String = "/trigger"): VilPipeline {
        nodes[nodeName] = Node("http_sink", port, path)
        if (nodeName !in nodeOrder) nodeOrder.add(nodeName)
        this.port = port
        return this
    }

    fun source(nodeName: String, url: String, format: String = "sse",
               jsonTap: String? = null, dialect: String? = null): VilPipeline {
        nodes[nodeName] = Node("http_source", url = url, format = format, jsonTap = jsonTap, dialect = dialect)
        if (nodeName !in nodeOrder) nodeOrder.add(nodeName)
        return this
    }

    fun route(from: String, to: String, mode: String): VilPipeline {
        routes.add(Route(from, to, mode))
        return this
    }

    fun toYaml(): String {
        val sb = StringBuilder()
        sb.appendLine("vil_version: \"6.0.0\"")
        sb.appendLine("name: $name")
        sb.appendLine("port: $port")
        sb.appendLine("token: shm")
        if (nodeOrder.isNotEmpty()) {
            sb.appendLine(); sb.appendLine("nodes:")
            for (n in nodeOrder) {
                val nd = nodes[n]!!
                sb.appendLine("  $n:")
                sb.appendLine("    type: ${nd.type}")
                if (nd.port != 0) sb.appendLine("    port: ${nd.port}")
                nd.path?.let { sb.appendLine("    path: \"$it\"") }
                nd.url?.let { sb.appendLine("    url: \"$it\"") }
                nd.format?.let { sb.appendLine("    format: $it") }
                nd.jsonTap?.let { sb.appendLine("    json_tap: \"$it\"") }
                nd.dialect?.let { sb.appendLine("    dialect: $it") }
            }
        }
        if (routes.isNotEmpty()) {
            sb.appendLine(); sb.appendLine("routes:")
            for (r in routes) {
                sb.appendLine("  - from: ${r.from}")
                sb.appendLine("    to: ${r.to}")
                sb.appendLine("    mode: ${r.mode}")
            }
        }
        return sb.toString()
    }

    fun compile() {
        val yaml = toYaml()
        if (System.getenv("VIL_COMPILE_MODE") == "manifest") { print(yaml); return }
        println("  Compiling: $name")
        print(yaml)
    }
}

class VilServer(private val name: String, private val port: Int) {
    private val services = mutableListOf<ServiceProcess>()
    private data class SemType(val name: String, val kind: String)
    private val semanticTypes = mutableListOf<SemType>()

    fun service(svc: ServiceProcess): VilServer { services.add(svc); return this }
    fun semanticType(name: String, kind: String): VilServer { semanticTypes.add(SemType(name, kind)); return this }

    fun toYaml(): String {
        val sb = StringBuilder()
        sb.appendLine("vil_version: \"6.0.0\"")
        sb.appendLine("name: $name")
        sb.appendLine("port: $port")
        if (semanticTypes.isNotEmpty()) {
            sb.appendLine("semantic_types:")
            for (st in semanticTypes) { sb.appendLine("  - name: ${st.name}"); sb.appendLine("    kind: ${st.kind}") }
        }
        if (services.isNotEmpty()) {
            sb.appendLine("services:")
            for (svc in services) {
                sb.appendLine("  - name: ${svc.name}")
                sb.appendLine("    prefix: \"${svc.prefix}\"")
                if (svc.endpoints.isNotEmpty()) {
                    sb.appendLine("    endpoints:")
                    for (ep in svc.endpoints) {
                        sb.appendLine("      - method: ${ep.method}")
                        sb.appendLine("        path: \"${ep.path}\"")
                        sb.appendLine("        handler: ${ep.handler}")
                    }
                }
            }
        }
        return sb.toString()
    }

    fun compile() {
        val yaml = toYaml()
        if (System.getenv("VIL_COMPILE_MODE") == "manifest") { print(yaml); return }
        println("  Compiling: $name")
        print(yaml)
    }
}
