package dev.vil;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Built-in SSE client for VIL — collects streaming SSE responses.
 *
 * <p>Supports all major AI provider dialects (OpenAI, Anthropic, Ollama,
 * Cohere, Gemini) plus custom dialects and W3C standard SSE.
 *
 * <p>Example:
 * <pre>{@code
 * String content = new SseCollect("http://localhost:4545/v1/chat/completions")
 *     .dialect(SseDialect.openai())
 *     .bearerToken(System.getenv("OPENAI_API_KEY"))
 *     .postJson(requestBody);
 * }</pre>
 */
public class SseCollect {

    private final String url;
    private SseDialect dialect;
    private final Map<String, String> headers = new LinkedHashMap<>();

    public SseCollect(String url) {
        this.url = url;
        this.dialect = SseDialect.openai();
        this.headers.put("Content-Type", "application/json");
        this.headers.put("Accept", "text/event-stream");
    }

    // =========================================================================
    // Builder methods
    // =========================================================================

    /** Set SSE dialect. */
    public SseCollect dialect(SseDialect d) {
        this.dialect = d;
        return this;
    }

    /** Set Bearer token auth (OpenAI, Cohere, Gemini). */
    public SseCollect bearerToken(String token) {
        if (token != null && !token.isEmpty()) {
            this.headers.put("Authorization", "Bearer " + token);
        }
        return this;
    }

    /** Set API key header auth (Anthropic: x-api-key). */
    public SseCollect apiKeyHeader(String headerName, String apiKey) {
        if (apiKey != null && !apiKey.isEmpty()) {
            this.headers.put(headerName, apiKey);
        }
        return this;
    }

    /** Set custom header. */
    public SseCollect header(String key, String value) {
        this.headers.put(key, value);
        return this;
    }

    /** Override done marker on current dialect. */
    public SseCollect doneMarker(String marker) {
        this.dialect.doneMarker(marker);
        return this;
    }

    /** Override JSON tap path on current dialect. */
    public SseCollect jsonTap(String path) {
        this.dialect.jsonTap(path);
        return this;
    }

    /** Set done detection via JSON field (on current dialect). */
    public SseCollect doneJsonField(String field, String value) {
        this.dialect.doneJsonField(field, value);
        return this;
    }

    // =========================================================================
    // Execute
    // =========================================================================

    /**
     * POST JSON body and collect SSE stream content.
     *
     * @param jsonBody JSON string to POST.
     * @return Collected content from SSE stream.
     * @throws Exception on network or parsing error.
     */
    public String postJson(String jsonBody) throws Exception {
        HttpURLConnection conn = (HttpURLConnection) new URL(url).openConnection();
        conn.setRequestMethod("POST");
        conn.setDoOutput(true);

        for (Map.Entry<String, String> h : headers.entrySet()) {
            conn.setRequestProperty(h.getKey(), h.getValue());
        }

        byte[] body = jsonBody.getBytes(StandardCharsets.UTF_8);
        conn.setFixedLengthStreamingMode(body.length);
        try (OutputStream os = conn.getOutputStream()) {
            os.write(body);
        }

        int status = conn.getResponseCode();
        if (status != 200) {
            throw new RuntimeException("SSE request failed with status " + status);
        }

        StringBuilder content = new StringBuilder();
        String doneMarker = dialect.getDoneMarker();
        String tapPath = dialect.getJsonTap();
        String doneField = dialect.getDoneJsonField();
        String doneValue = dialect.getDoneJsonValue();

        try (BufferedReader reader = new BufferedReader(
                new InputStreamReader(conn.getInputStream(), StandardCharsets.UTF_8))) {

            String line;
            while ((line = reader.readLine()) != null) {
                // Check done marker in raw line
                if (!doneMarker.isEmpty() && line.contains(doneMarker)) {
                    break;
                }

                // Parse SSE data lines
                if (line.startsWith("data: ")) {
                    String data = line.substring(6);

                    // Check done marker in data
                    if (!doneMarker.isEmpty() && data.equals(doneMarker)) {
                        break;
                    }

                    // Check done JSON field
                    if (!doneField.isEmpty() && data.contains("\"" + doneField + "\":" + doneValue)) {
                        break;
                    }
                    if (!doneField.isEmpty() && data.contains("\"" + doneField + "\": " + doneValue)) {
                        break;
                    }

                    // Extract content via json_tap (simple path extraction)
                    if (!tapPath.isEmpty()) {
                        String extracted = extractJsonPath(data, tapPath);
                        if (extracted != null) {
                            content.append(extracted);
                        }
                    } else {
                        // No tap — append raw data
                        content.append(data);
                    }
                }
            }
        } finally {
            conn.disconnect();
        }

        return content.toString();
    }

    // =========================================================================
    // JSON path extraction (simple, no external deps)
    // =========================================================================

    /**
     * Extract value from JSON string using a simple dot-notation path.
     * Supports: "key", "key.subkey", "array[0].field"
     */
    static String extractJsonPath(String json, String path) {
        try {
            String current = json.trim();
            String[] parts = path.split("\\.");

            for (String part : parts) {
                // Handle array index: "choices[0]"
                int bracketIdx = part.indexOf('[');
                if (bracketIdx >= 0) {
                    String key = part.substring(0, bracketIdx);
                    int arrayIdx = Integer.parseInt(
                        part.substring(bracketIdx + 1, part.indexOf(']')));

                    // Navigate to key
                    current = findJsonValue(current, key);
                    if (current == null) return null;

                    // Navigate to array element
                    current = findArrayElement(current, arrayIdx);
                    if (current == null) return null;
                } else {
                    current = findJsonValue(current, part);
                    if (current == null) return null;
                }
            }

            // Remove quotes if string value
            if (current.startsWith("\"") && current.endsWith("\"")) {
                return current.substring(1, current.length() - 1)
                    .replace("\\n", "\n")
                    .replace("\\t", "\t")
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\");
            }
            // null values
            if (current.equals("null")) return null;

            return current;
        } catch (Exception e) {
            return null;
        }
    }

    private static String findJsonValue(String json, String key) {
        String searchKey = "\"" + key + "\"";
        int keyIdx = json.indexOf(searchKey);
        if (keyIdx < 0) return null;

        int colonIdx = json.indexOf(':', keyIdx + searchKey.length());
        if (colonIdx < 0) return null;

        int valueStart = colonIdx + 1;
        while (valueStart < json.length() && json.charAt(valueStart) == ' ') {
            valueStart++;
        }

        return extractJsonToken(json, valueStart);
    }

    private static String findArrayElement(String json, int index) {
        int pos = 0;
        while (pos < json.length() && json.charAt(pos) != '[') pos++;
        pos++; // skip '['

        for (int i = 0; i < index; i++) {
            while (pos < json.length() && json.charAt(pos) == ' ') pos++;
            String skip = extractJsonToken(json, pos);
            if (skip == null) return null;
            pos += skip.length();
            while (pos < json.length() && (json.charAt(pos) == ',' || json.charAt(pos) == ' ')) pos++;
        }

        while (pos < json.length() && json.charAt(pos) == ' ') pos++;
        return extractJsonToken(json, pos);
    }

    private static String extractJsonToken(String json, int start) {
        if (start >= json.length()) return null;
        char c = json.charAt(start);

        if (c == '"') {
            // String
            int end = start + 1;
            while (end < json.length()) {
                if (json.charAt(end) == '\\') { end += 2; continue; }
                if (json.charAt(end) == '"') break;
                end++;
            }
            return json.substring(start, end + 1);
        } else if (c == '{') {
            // Object
            int depth = 1;
            int end = start + 1;
            while (end < json.length() && depth > 0) {
                if (json.charAt(end) == '{') depth++;
                else if (json.charAt(end) == '}') depth--;
                else if (json.charAt(end) == '"') {
                    end++;
                    while (end < json.length() && json.charAt(end) != '"') {
                        if (json.charAt(end) == '\\') end++;
                        end++;
                    }
                }
                end++;
            }
            return json.substring(start, end);
        } else if (c == '[') {
            // Array
            int depth = 1;
            int end = start + 1;
            while (end < json.length() && depth > 0) {
                if (json.charAt(end) == '[') depth++;
                else if (json.charAt(end) == ']') depth--;
                else if (json.charAt(end) == '"') {
                    end++;
                    while (end < json.length() && json.charAt(end) != '"') {
                        if (json.charAt(end) == '\\') end++;
                        end++;
                    }
                }
                end++;
            }
            return json.substring(start, end);
        } else {
            // Number, boolean, null
            int end = start;
            while (end < json.length() && json.charAt(end) != ',' &&
                   json.charAt(end) != '}' && json.charAt(end) != ']' &&
                   json.charAt(end) != ' ' && json.charAt(end) != '\n') {
                end++;
            }
            return json.substring(start, end);
        }
    }
}
