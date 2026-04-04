// 010-basic-websocket-chat — C# SDK equivalent
// Compile: vil compile --from csharp --input 010-basic-websocket-chat/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("websocket-chat", 8080);
var chat = new ServiceProcess("chat");
chat.Endpoint("GET", "/", "index");
chat.Endpoint("GET", "/ws", "ws_handler");
chat.Endpoint("GET", "/stats", "stats");
server.Service(chat);
server.Compile();
