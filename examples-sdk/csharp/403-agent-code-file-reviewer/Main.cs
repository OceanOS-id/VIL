// 403-agent-code-file-reviewer — C# SDK equivalent
// Compile: vil compile --from csharp --input 403-agent-code-file-reviewer/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("code-file-reviewer-agent", 3122);
var code_review_agent = new ServiceProcess("code-review-agent");
code_review_agent.Endpoint("POST", "/code-review", "code_review_handler");
server.Service(code_review_agent);
server.Compile();
