// 303-rag-hybrid-exact-semantic — C# SDK equivalent
// Compile: vil compile --from csharp --input 303-rag-hybrid-exact-semantic/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("rag-hybrid-exact-semantic", 3112);
var rag_hybrid = new ServiceProcess("rag-hybrid");
rag_hybrid.Endpoint("POST", "/hybrid", "hybrid_handler");
server.Service(rag_hybrid);
server.Compile();
