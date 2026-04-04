// 037-basic-vilmodel-derive — C# SDK equivalent
// Compile: vil compile --from csharp --input 037-basic-vilmodel-derive/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("insurance-claim-processing", 8080);
var claims = new ServiceProcess("claims");
claims.Endpoint("POST", "/claims/submit", "submit_claim");
claims.Endpoint("GET", "/claims/sample", "sample_claim");
server.Service(claims);
server.Compile();
