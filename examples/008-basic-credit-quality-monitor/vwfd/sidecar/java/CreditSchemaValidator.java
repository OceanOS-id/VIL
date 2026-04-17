import java.io.*;

/**
 * Credit Schema Validator — VIL Sidecar (line-delimited JSON protocol).
 *
 * Protocol: read one JSON line from stdin → validate → write one JSON line to stdout → loop.
 * Process stays alive for reuse across requests.
 */
public class CreditSchemaValidator {
    public static void main(String[] args) throws Exception {
        BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
        BufferedWriter writer = new BufferedWriter(new OutputStreamWriter(System.out));

        String line;
        while ((line = reader.readLine()) != null) {
            String input = line.trim();
            if (input.isEmpty()) continue;

            String[] required = {"nik", "nama_lengkap", "jumlah_kredit", "kolektabilitas"};
            StringBuilder missing = new StringBuilder();
            for (String f : required) {
                if (!input.contains("\"" + f + "\"")) {
                    if (missing.length() > 0) missing.append(",");
                    missing.append("\"").append(f).append("\"");
                }
            }

            if (missing.length() > 0) {
                writer.write(String.format("{\"valid\":false,\"missing\":[%s]}", missing));
            } else {
                writer.write(String.format("{\"valid\":true,\"fields_checked\":%d}", required.length));
            }
            writer.newLine();
            writer.flush();
        }
    }
}
