// 003-basic-hello-server — C# SDK equivalent
// Compile: vil compile --from csharp --input 003-basic-hello-server/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("vil-basic-hello-server", 8080);
var gw = new ServiceProcess("gw");
gw.Endpoint("POST", "/transform", "transform");
gw.Endpoint("POST", "/echo", "echo");
gw.Endpoint("GET", "/health", "health");
server.Service(gw);
server.Compile();
