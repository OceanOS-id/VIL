package dev.vil;

/**
 * SSE dialect configuration for different AI providers.
 *
 * <p>Each provider uses slightly different SSE conventions for streaming.
 * SseDialect captures these differences so the VIL pipeline can handle them
 * transparently.
 *
 * <p>Built-in dialects: OpenAI, Anthropic, Ollama, Cohere, Gemini, Standard (W3C).
 *
 * <p>Example:
 * <pre>{@code
 * SseDialect dialect = SseDialect.openai();
 * // or custom:
 * SseDialect custom = new SseDialect("myProvider", "[END]", "result.text")
 *     .doneJsonField("finished", "true");
 * }</pre>
 */
public class SseDialect {

    private final String name;
    private String doneMarker;
    private String jsonTap;
    private String doneJsonField = "";
    private String doneJsonValue = "";

    public SseDialect(String name, String doneMarker, String jsonTap) {
        this.name = name;
        this.doneMarker = doneMarker;
        this.jsonTap = jsonTap;
    }

    // =========================================================================
    // Built-in dialects
    // =========================================================================

    /** OpenAI / OpenAI-compatible (GPT-4, GPT-3.5, vLLM, etc.) */
    public static SseDialect openai() {
        return new SseDialect("openai", "[DONE]", "choices[0].delta.content");
    }

    /** Anthropic Claude (Messages API) */
    public static SseDialect anthropic() {
        return new SseDialect("anthropic", "event: message_stop", "delta.text");
    }

    /** Ollama (local models) */
    public static SseDialect ollama() {
        SseDialect d = new SseDialect("ollama", "\"done\":true", "message.content");
        d.doneJsonField = "done";
        d.doneJsonValue = "true";
        return d;
    }

    /** Cohere (Command R, etc.) */
    public static SseDialect cohere() {
        SseDialect d = new SseDialect("cohere", "\"is_finished\":true", "text");
        d.doneJsonField = "is_finished";
        d.doneJsonValue = "true";
        return d;
    }

    /** Google Gemini */
    public static SseDialect gemini() {
        return new SseDialect("gemini", "[DONE]", "candidates[0].content.parts[0].text");
    }

    /** W3C Standard SSE (no done marker, EOF terminates) */
    public static SseDialect standard() {
        return new SseDialect("standard", "", "");
    }

    // =========================================================================
    // Builder methods for custom dialects
    // =========================================================================

    /** Override done marker. */
    public SseDialect doneMarker(String marker) {
        this.doneMarker = marker;
        return this;
    }

    /** Override JSON tap path. */
    public SseDialect jsonTap(String tap) {
        this.jsonTap = tap;
        return this;
    }

    /** Set done detection via JSON field value (alternative to done marker). */
    public SseDialect doneJsonField(String field, String value) {
        this.doneJsonField = field;
        this.doneJsonValue = value;
        return this;
    }

    // =========================================================================
    // Getters
    // =========================================================================

    public String getName() { return name; }
    public String getDoneMarker() { return doneMarker; }
    public String getJsonTap() { return jsonTap; }
    public String getDoneJsonField() { return doneJsonField; }
    public String getDoneJsonValue() { return doneJsonValue; }

    @Override
    public String toString() {
        return "SseDialect{name='" + name + "', doneMarker='" + doneMarker +
               "', jsonTap='" + jsonTap + "'}";
    }
}
