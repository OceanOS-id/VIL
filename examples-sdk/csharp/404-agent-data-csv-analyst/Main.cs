// 404-agent-data-csv-analyst — C# SDK equivalent
// Compile: vil compile --from csharp --input 404-agent-data-csv-analyst/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("csv-analyst-agent", 3123);
var csv_analyst_agent = new ServiceProcess("csv-analyst-agent");
csv_analyst_agent.Endpoint("POST", "/csv-analyze", "csv_analyze_handler");
server.Service(csv_analyst_agent);
server.Compile();
