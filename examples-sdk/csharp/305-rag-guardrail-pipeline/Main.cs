// 305-rag-guardrail-pipeline — C# SDK equivalent
// Compile: vil compile --from csharp --input 305-rag-guardrail-pipeline/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("rag-guardrail-pipeline", 3114);
var rag_guardrail = new ServiceProcess("rag-guardrail");
rag_guardrail.Endpoint("POST", "/safe-rag", "safe_rag_handler");
server.Service(rag_guardrail);
server.Compile();
