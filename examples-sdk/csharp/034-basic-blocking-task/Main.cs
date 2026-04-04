// 034-basic-blocking-task — C# SDK equivalent
// Compile: vil compile --from csharp --input 034-basic-blocking-task/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("credit-risk-scoring-engine", 8080);
var risk_engine = new ServiceProcess("risk-engine");
risk_engine.Endpoint("POST", "/risk/assess", "assess_risk");
risk_engine.Endpoint("GET", "/risk/health", "risk_health");
server.Service(risk_engine);
server.Compile();
