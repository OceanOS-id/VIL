// 405-agent-react-multi-tool — Java SDK equivalent
// Compile: vil compile --from java --input 405-agent-react-multi-tool/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("react-multi-tool-agent", 3124);
        ServiceProcess react_agent = new ServiceProcess("react-agent");
        react_agent.endpoint("POST", "/react", "react_handler");
        server.service(react_agent);
        server.compile();
    }
}
