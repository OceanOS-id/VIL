// 206-llm-decision-routing — C# SDK equivalent
// Compile: vil compile --from csharp --input 206-llm-decision-routing/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("insurance-underwriting-ai", 3116);
var underwriter = new ServiceProcess("underwriter");
server.Service(underwriter);
server.Compile();
