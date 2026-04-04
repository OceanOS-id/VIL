// 024-basic-llm-chat — Java SDK equivalent
// Compile: vil compile --from java --input 024-basic-llm-chat/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("llm-chat", 8080);
        ServiceProcess chat = new ServiceProcess("chat");
        chat.endpoint("POST", "/chat", "chat_handler");
        server.service(chat);
        server.compile();
    }
}
