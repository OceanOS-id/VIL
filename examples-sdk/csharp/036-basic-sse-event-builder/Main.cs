// 036-basic-sse-event-builder — C# SDK equivalent
// Compile: vil compile --from csharp --input 036-basic-sse-event-builder/Main.cs --release

#r "sdk/csharp/Vil.cs"

var server = new VilServer("stock-market-ticker", 8080);
var ticker = new ServiceProcess("ticker");
ticker.Endpoint("GET", "/stream", "ticker_stream");
ticker.Endpoint("GET", "/info", "ticker_info");
server.Service(ticker);
server.Compile();
