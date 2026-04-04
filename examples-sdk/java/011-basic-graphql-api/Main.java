// 011-basic-graphql-api — Java SDK equivalent
// Compile: vil compile --from java --input 011-basic-graphql-api/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("graphql-api", 8080);
        ServiceProcess graphql = new ServiceProcess("graphql");
        graphql.endpoint("GET", "/", "index");
        graphql.endpoint("GET", "/schema", "schema_info");
        graphql.endpoint("GET", "/entities", "list_entities");
        graphql.endpoint("POST", "/query", "graphql_query");
        server.service(graphql);
        server.compile();
    }
}
