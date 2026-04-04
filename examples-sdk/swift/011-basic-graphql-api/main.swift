// 011-basic-graphql-api — Swift SDK equivalent
// Compile: vil compile --from swift --input 011-basic-graphql-api/main.swift --release

let server = VilServer(name: "graphql-api", port: 8080)
let graphql = ServiceProcess(name: "graphql")
graphql.endpoint(method: "GET", path: "/", handler: "index")
graphql.endpoint(method: "GET", path: "/schema", handler: "schema_info")
graphql.endpoint(method: "GET", path: "/entities", handler: "list_entities")
graphql.endpoint(method: "POST", path: "/query", handler: "graphql_query")
server.service(graphql)
server.compile()
