// 025-basic-rag-service — C# SDK equivalent
// Compile: vil compile --from csharp --input 025-basic-rag-service/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("rag-service", 3091);
var rag = new ServiceProcess("rag");
rag.Endpoint("POST", "/rag", "rag_handler");
server.Service(rag);
server.Compile();
