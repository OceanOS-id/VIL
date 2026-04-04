// 033-basic-shm-write-through — C# SDK equivalent
// Compile: vil compile --from csharp --input 033-basic-shm-write-through/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("realtime-analytics-dashboard", 8080);
var catalog = new ServiceProcess("catalog");
catalog.Endpoint("POST", "/catalog/search", "catalog_search");
catalog.Endpoint("GET", "/catalog/health", "catalog_health");
server.Service(catalog);
server.Compile();
