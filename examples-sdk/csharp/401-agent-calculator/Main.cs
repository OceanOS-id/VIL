// 401-agent-calculator — C# SDK equivalent
// Compile: vil compile --from csharp --input 401-agent-calculator/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("calculator-agent", 3120);
var calc_agent = new ServiceProcess("calc-agent");
calc_agent.Endpoint("POST", "/calc", "calc_handler");
server.Service(calc_agent);
server.Compile();
