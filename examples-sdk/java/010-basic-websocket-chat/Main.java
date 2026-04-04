// 010-basic-websocket-chat — Java SDK equivalent
// Compile: vil compile --from java --input 010-basic-websocket-chat/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("websocket-chat", 8080);
        ServiceProcess chat = new ServiceProcess("chat");
        chat.endpoint("GET", "/", "index");
        chat.endpoint("GET", "/ws", "ws_handler");
        chat.endpoint("GET", "/stats", "stats");
        server.service(chat);
        server.compile();
    }
}
