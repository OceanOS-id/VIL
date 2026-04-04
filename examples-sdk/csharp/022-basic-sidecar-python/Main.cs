// 022-basic-sidecar-python — C# SDK equivalent
// Compile: vil compile --from csharp --input 022-basic-sidecar-python/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("sidecar-python-example", 8080);
var fraud = new ServiceProcess("fraud");
fraud.Endpoint("GET", "/status", "fraud_status");
fraud.Endpoint("POST", "/check", "fraud_check");
server.Service(fraud);
var root = new ServiceProcess("root");
root.Endpoint("GET", "/", "index");
server.Service(root);
server.Compile();
