// 011-basic-graphql-api — Zig SDK equivalent
// Compile: vil compile --from zig --input 011-basic-graphql-api/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("graphql-api", 8080);
    var graphql = vil.Service.init("graphql");
    graphql.endpoint("GET", "/", "index");
    graphql.endpoint("GET", "/schema", "schema_info");
    graphql.endpoint("GET", "/entities", "list_entities");
    graphql.endpoint("POST", "/query", "graphql_query");
    server.service(&graphql);
    server.compile();
}
