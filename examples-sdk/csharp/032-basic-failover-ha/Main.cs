// 032-basic-failover-ha — C# SDK equivalent
// Compile: vil compile --from csharp --input 032-basic-failover-ha/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("payment-gateway-ha", 8080);
var primary = new ServiceProcess("primary");
primary.Endpoint("GET", "/health", "primary_health");
primary.Endpoint("POST", "/charge", "primary_charge");
server.Service(primary);
var backup = new ServiceProcess("backup");
backup.Endpoint("GET", "/health", "backup_health");
backup.Endpoint("POST", "/charge", "backup_charge");
server.Service(backup);
server.Compile();
