// 304-rag-citation-extraction — C# SDK equivalent
// Compile: vil compile --from csharp --input 304-rag-citation-extraction/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("rag-citation-extraction", 3113);
var rag_citation = new ServiceProcess("rag-citation");
rag_citation.Endpoint("POST", "/cited-rag", "cited_rag_handler");
server.Service(rag_citation);
server.Compile();
