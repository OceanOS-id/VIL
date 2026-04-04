// 301-rag-basic-vector-search — C# SDK equivalent
// Compile: vil compile --from csharp --input 301-rag-basic-vector-search/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("rag-basic-vector-search", 3110);
var rag_basic = new ServiceProcess("rag-basic");
rag_basic.Endpoint("POST", "/rag", "rag_handler");
server.Service(rag_basic);
server.Compile();
