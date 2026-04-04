// 011-basic-graphql-api — C# SDK equivalent
// Compile: vil compile --from csharp --input 011-basic-graphql-api/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("graphql-api", 8080);
var graphql = new ServiceProcess("graphql");
graphql.Endpoint("GET", "/", "index");
graphql.Endpoint("GET", "/schema", "schema_info");
graphql.Endpoint("GET", "/entities", "list_entities");
graphql.Endpoint("POST", "/query", "graphql_query");
server.Service(graphql);
server.Compile();
