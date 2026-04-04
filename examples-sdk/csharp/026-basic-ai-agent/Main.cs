// 026-basic-ai-agent — C# SDK equivalent
// Compile: vil compile --from csharp --input 026-basic-ai-agent/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("ai-agent", 8080);
var agent = new ServiceProcess("agent");
agent.Endpoint("POST", "/agent", "agent_handler");
server.Service(agent);
server.Compile();
