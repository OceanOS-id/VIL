// 010 — WebSocket Chat
// Equivalent to: examples/010-basic-websocket-chat (Rust)
// Compile: vil compile --from java --input 010/Main.java --release
package dev.vil.examples;

import dev.vil.VilServer;
import dev.vil.ServiceProcess;
import java.util.Map;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("websocket-chat", 8080);

        // -- WebSocket events -------------------------------------------------
        server.wsEvent("chat_message", "chat.message", Map.of(
            "from", "String", "message", "String", "timestamp", "String"
        ));
        server.wsEvent("user_joined", "chat.presence", Map.of(
            "username", "String"
        ));
        server.wsEvent("user_left", "chat.presence", Map.of(
            "username", "String"
        ));

        // -- ServiceProcess: chat (prefix: /api/chat) -------------------------
        ServiceProcess chat = new ServiceProcess("chat");
        chat.endpoint("GET", "/", "index");
        chat.endpointWs("GET", "/ws", "ws_handler");
        chat.endpoint("GET", "/stats", "stats");
        server.service(chat, "/api/chat");

        // -- Emit / compile ---------------------------------------------------
        if ("manifest".equals(System.getenv("VIL_COMPILE_MODE"))) {
            System.out.println(server.toYaml());
        } else {
            server.compile();
        }
    }
}
