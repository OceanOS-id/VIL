// 803-trigger-webhook-receiver — C# SDK equivalent
// Compile: vil compile --from csharp --input 803-trigger-webhook-receiver/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
