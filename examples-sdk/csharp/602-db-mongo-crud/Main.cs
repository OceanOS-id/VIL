// 602-db-mongo-crud — C# SDK equivalent
// Compile: vil compile --from csharp --input 602-db-mongo-crud/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
