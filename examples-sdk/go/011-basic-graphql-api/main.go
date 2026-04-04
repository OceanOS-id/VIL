// 011-basic-graphql-api — Go SDK equivalent
// Compile: vil compile --from go --input 011-basic-graphql-api/main.go --release
package main

import vil "github.com/OceanOS-id/vil-go"

func main() {
	s := vil.NewServer("graphql-api", 8080)

	graphql := vil.NewService("graphql")
	graphql.Endpoint("GET", "/", "index")
	graphql.Endpoint("GET", "/schema", "schema_info")
	graphql.Endpoint("GET", "/entities", "list_entities")
	graphql.Endpoint("POST", "/query", "graphql_query")
	s.Service(graphql)

	s.Compile()
}
