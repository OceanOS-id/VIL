// 404-agent-data-csv-analyst — Java SDK equivalent
// Compile: vil compile --from java --input 404-agent-data-csv-analyst/Main.java --release

import dev.vil.*;

public class Main {
    public static void main(String[] args) {
        VilServer server = new VilServer("csv-analyst-agent", 3123);
        ServiceProcess csv_analyst_agent = new ServiceProcess("csv-analyst-agent");
        csv_analyst_agent.endpoint("POST", "/csv-analyze", "csv_analyze_handler");
        server.service(csv_analyst_agent);
        server.compile();
    }
}
