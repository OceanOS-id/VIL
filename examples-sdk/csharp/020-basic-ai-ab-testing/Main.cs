// 020-basic-ai-ab-testing — C# SDK equivalent
// Compile: vil compile --from csharp --input 020-basic-ai-ab-testing/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("ai-ab-testing-gateway", 8080);
var ab = new ServiceProcess("ab");
ab.Endpoint("POST", "/infer", "infer");
ab.Endpoint("GET", "/metrics", "metrics");
ab.Endpoint("POST", "/config", "update_config");
server.Service(ab);
var root = new ServiceProcess("root");
root.Endpoint("GET", "/", "index");
server.Service(root);
server.Compile();
