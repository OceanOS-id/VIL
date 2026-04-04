// 011-basic-graphql-api — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 011-basic-graphql-api/main.kt --release

fun main() {
    val server = VilServer("graphql-api", 8080)
    val graphql = ServiceProcess("graphql")
    graphql.endpoint("GET", "/", "index")
    graphql.endpoint("GET", "/schema", "schema_info")
    graphql.endpoint("GET", "/entities", "list_entities")
    graphql.endpoint("POST", "/query", "graphql_query")
    server.service(graphql)
    server.compile()
}
