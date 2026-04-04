// 001b-vilapp-ai-gw-benchmark — C# SDK equivalent
// Compile: vil compile --from csharp --input 001b-vilapp-ai-gw-benchmark/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("ai-gw-bench", 3081);
var gw = new ServiceProcess("gw");
server.Service(gw);
server.Compile();
