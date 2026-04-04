// 036-basic-sse-event-builder — Zig SDK equivalent
// Compile: vil compile --from zig --input 036-basic-sse-event-builder/main.zig --release

const vil = @import("vil");

pub fn main() void {
    var server = vil.Server.init("stock-market-ticker", 8080);
    var ticker = vil.Service.init("ticker");
    ticker.endpoint("GET", "/stream", "ticker_stream");
    ticker.endpoint("GET", "/info", "ticker_info");
    server.service(&ticker);
    server.compile();
}
