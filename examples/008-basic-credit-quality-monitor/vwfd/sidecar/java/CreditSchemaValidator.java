import java.io.*;
public class CreditSchemaValidator {
    public static void main(String[] args) throws Exception {
        BufferedReader r = new BufferedReader(new InputStreamReader(System.in));
        StringBuilder sb = new StringBuilder();
        String line; while ((line = r.readLine()) != null) sb.append(line);
        String input = sb.toString();
        
        String[] required = {"nik", "nama_lengkap", "jumlah_kredit", "kolektabilitas"};
        StringBuilder missing = new StringBuilder();
        for (String f : required) {
            if (!input.contains("\"" + f + "\"")) {
                if (missing.length() > 0) missing.append(",");
                missing.append("\"").append(f).append("\"");
            }
        }
        if (missing.length() > 0) {
            System.out.printf("{\"valid\":false,\"missing\":[%s]}%n", missing);
        } else {
            System.out.printf("{\"valid\":true,\"fields_checked\":%d}%n", required.length);
        }
    }
}
