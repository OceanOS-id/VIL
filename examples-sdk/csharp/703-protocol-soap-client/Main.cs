// 703-protocol-soap-client — C# SDK equivalent
// Compile: vil compile --from csharp --input 703-protocol-soap-client/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
