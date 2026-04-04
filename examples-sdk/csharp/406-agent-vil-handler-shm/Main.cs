// 406-agent-vil-handler-shm — C# SDK equivalent
// Compile: vil compile --from csharp --input 406-agent-vil-handler-shm/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("fraud-detection-agent", 3126);
var fraud_agent = new ServiceProcess("fraud-agent");
fraud_agent.Endpoint("POST", "/detect", "detect_fraud");
fraud_agent.Endpoint("GET", "/health", "health");
server.Service(fraud_agent);
server.Compile();
