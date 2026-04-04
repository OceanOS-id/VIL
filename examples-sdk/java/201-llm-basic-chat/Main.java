// 201-llm-basic-chat — Java SDK equivalent
// Compile: vil compile --from java --input 201-llm-basic-chat/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("llm-basic-chat", 3100);
        ServiceProcess chat = new ServiceProcess("chat");
        chat.endpoint("POST", "/chat", "chat_handler");
        server.service(chat);
        server.compile();
    }
}
