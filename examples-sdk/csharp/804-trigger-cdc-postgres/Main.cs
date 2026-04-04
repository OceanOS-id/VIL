// 804-trigger-cdc-postgres — C# SDK equivalent
// Compile: vil compile --from csharp --input 804-trigger-cdc-postgres/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
