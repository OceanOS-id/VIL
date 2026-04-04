// 002-basic-vilapp-gateway — C# SDK equivalent
// Compile: vil compile --from csharp --input 002-basic-vilapp-gateway/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("vil-app-gateway", 3081);
var gw = new ServiceProcess("gw");
gw.Endpoint("POST", "/trigger", "trigger_handler");
server.Service(gw);
server.Compile();
