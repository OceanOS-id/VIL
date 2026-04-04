// 023-basic-hybrid-wasm-sidecar — C# SDK equivalent
// Compile: vil compile --from csharp --input 023-basic-hybrid-wasm-sidecar/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("hybrid-pipeline", 8080);
var pipeline = new ServiceProcess("pipeline");
pipeline.Endpoint("GET", "/", "index");
pipeline.Endpoint("POST", "/validate", "validate_order");
pipeline.Endpoint("POST", "/price", "calculate_price");
pipeline.Endpoint("POST", "/fraud", "fraud_check");
pipeline.Endpoint("POST", "/order", "process_order");
server.Service(pipeline);
server.Compile();
