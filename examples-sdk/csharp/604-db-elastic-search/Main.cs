// 604-db-elastic-search — C# SDK equivalent
// Compile: vil compile --from csharp --input 604-db-elastic-search/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
