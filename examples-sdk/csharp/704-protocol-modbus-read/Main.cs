// 704-protocol-modbus-read — C# SDK equivalent
// Compile: vil compile --from csharp --input 704-protocol-modbus-read/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("app", 8080);
server.Compile();
