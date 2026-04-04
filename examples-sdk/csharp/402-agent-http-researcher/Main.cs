// 402-agent-http-researcher — C# SDK equivalent
// Compile: vil compile --from csharp --input 402-agent-http-researcher/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("http-researcher-agent", 3121);
var research_agent = new ServiceProcess("research-agent");
research_agent.Endpoint("POST", "/research", "research_handler");
research_agent.Endpoint("GET", "/products", "products_handler");
server.Service(research_agent);
server.Compile();
