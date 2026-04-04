// 027-basic-vilserver-minimal — C# SDK equivalent
// Compile: vil compile --from csharp --input 027-basic-vilserver-minimal/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
