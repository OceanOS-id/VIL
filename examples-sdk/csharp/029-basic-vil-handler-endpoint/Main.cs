// 029-basic-vil-handler-endpoint — C# SDK equivalent
// Compile: vil compile --from csharp --input 029-basic-vil-handler-endpoint/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("macro-demo", 8080);
var demo = new ServiceProcess("demo");
demo.Endpoint("GET", "/plain", "plain_handler");
demo.Endpoint("GET", "/handled", "handled_handler");
demo.Endpoint("POST", "/endpoint", "endpoint_handler");
server.Service(demo);
server.Compile();
