// 036-basic-sse-event-builder — Java SDK equivalent
// Compile: vil compile --from java --input 036-basic-sse-event-builder/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("stock-market-ticker", 8080);
        ServiceProcess ticker = new ServiceProcess("ticker");
        ticker.endpoint("GET", "/stream", "ticker_stream");
        ticker.endpoint("GET", "/info", "ticker_info");
        server.service(ticker);
        server.compile();
    }
}
