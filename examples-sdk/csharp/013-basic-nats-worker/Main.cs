// 013-basic-nats-worker — C# SDK equivalent
// Compile: vil compile --from csharp --input 013-basic-nats-worker/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("nats-worker", 8080);
var nats = new ServiceProcess("nats");
nats.Endpoint("GET", "/nats/config", "nats_config");
nats.Endpoint("POST", "/nats/publish", "nats_publish");
nats.Endpoint("GET", "/nats/jetstream", "jetstream_info");
nats.Endpoint("GET", "/nats/kv", "kv_demo");
server.Service(nats);
var root = new ServiceProcess("root");
server.Service(root);
server.Compile();
