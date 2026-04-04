// 306-rag-ai-event-tracking — C# SDK equivalent
// Compile: vil compile --from csharp --input 306-rag-ai-event-tracking/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("customer-support-rag", 3116);
var support = new ServiceProcess("support");
support.Endpoint("POST", "/support/ask", "answer_question");
server.Service(support);
server.Compile();
