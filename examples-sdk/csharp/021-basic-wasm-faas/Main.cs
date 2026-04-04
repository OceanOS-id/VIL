// 021-basic-wasm-faas — C# SDK equivalent
// Compile: vil compile --from csharp --input 021-basic-wasm-faas/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("wasm-faas-example", 8080);
var wasm_faas = new ServiceProcess("wasm-faas");
wasm_faas.Endpoint("GET", "/", "index");
wasm_faas.Endpoint("GET", "/wasm/modules", "list_modules");
wasm_faas.Endpoint("POST", "/wasm/pricing", "invoke_pricing");
wasm_faas.Endpoint("POST", "/wasm/validation", "invoke_validation");
wasm_faas.Endpoint("POST", "/wasm/transform", "invoke_transform");
server.Service(wasm_faas);
server.Compile();
