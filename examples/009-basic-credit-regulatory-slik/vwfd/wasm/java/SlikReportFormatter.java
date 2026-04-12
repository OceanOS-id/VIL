/**
 * SLIK Report Formatter — Java WASM Module
 * Compile: javac SlikReportFormatter.java → TeaVM → .wasm
 * 
 * Formats credit data into OJK regulatory SLIK report format.
 */

import java.io.*;
import java.util.*;

public class SlikReportFormatter {
    
    public static String formatReport(String nik, String nama, int facilities, String kolektabilitas) {
        String reportId = "SLIK-" + nik;
        String riskLevel;
        int kol = Integer.parseInt(kolektabilitas);
        if (kol >= 5) riskLevel = "macet";
        else if (kol >= 3) riskLevel = "kurang_lancar";
        else riskLevel = "lancar";
        
        return String.format(
            "{\"report_id\":\"%s\",\"nama\":\"%s\",\"total_facilities\":%d," +
            "\"kolektabilitas\":%d,\"risk_level\":\"%s\",\"format\":\"OJK-v3\"," +
            "\"regulatory_body\":\"OJK\",\"report_type\":\"SLIK_INDIVIDUAL\"}",
            reportId, nama, facilities, kol, riskLevel
        );
    }
    
    public static void main(String[] args) throws Exception {
        BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
        StringBuilder sb = new StringBuilder();
        String line;
        while ((line = reader.readLine()) != null) sb.append(line);
        String input = sb.toString();
        
        // Simple JSON field extraction
        String nik = extractField(input, "nik");
        String nama = extractField(input, "nama");
        int facilities = extractInt(input, "facilities_count", 0);
        String kol = extractField(input, "kolektabilitas");
        if (kol.isEmpty()) kol = "1";
        
        System.out.println(formatReport(nik, nama, facilities, kol));
    }
    
    static String extractField(String json, String key) {
        String pattern = "\"" + key + "\":\"";
        int pos = json.indexOf(pattern);
        if (pos < 0) return "";
        int start = pos + pattern.length();
        int end = json.indexOf("\"", start);
        return end > start ? json.substring(start, end) : "";
    }
    
    static int extractInt(String json, String key, int def) {
        String pattern = "\"" + key + "\":";
        int pos = json.indexOf(pattern);
        if (pos < 0) return def;
        int start = pos + pattern.length();
        StringBuilder num = new StringBuilder();
        for (int i = start; i < json.length(); i++) {
            char c = json.charAt(i);
            if (Character.isDigit(c) || c == '-') num.append(c);
            else if (num.length() > 0) break;
        }
        try { return Integer.parseInt(num.toString()); }
        catch (Exception e) { return def; }
    }
}
