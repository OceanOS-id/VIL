// 006-basic-shm-extractor — C# SDK equivalent
// Compile: vil compile --from csharp --input 006-basic-shm-extractor/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("shm-extractor-demo", 8080);
var shm_demo = new ServiceProcess("shm-demo");
shm_demo.Endpoint("POST", "/ingest", "ingest");
shm_demo.Endpoint("POST", "/compute", "compute");
shm_demo.Endpoint("GET", "/shm-stats", "shm_stats");
shm_demo.Endpoint("GET", "/benchmark", "benchmark");
server.Service(shm_demo);
server.Compile();
