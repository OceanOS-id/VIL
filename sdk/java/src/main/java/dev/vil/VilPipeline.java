package dev.vil;

import java.io.File;
import java.io.FileWriter;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * VIL Pipeline — Declarative pipeline DSL -> YAML manifest -> native binary.
 *
 * <p>Generates a YAML manifest with nodes + routes sections.
 * {@code vil compile} transpiles it to a native Rust binary using
 * VIL Way patterns (ServiceCtx, ShmSlice).
 *
 * <p>No FFI, no JNI — pure YAML generation.
 *
 * <p>Example:
 * <pre>{@code
 * VilPipeline pipeline = new VilPipeline("ai-gateway");
 * pipeline.sink(3080, "/trigger", null);
 * pipeline.source("http://localhost:4545/v1/chat/completions",
 *     "sse", "choices[0].delta.content", null, null);
 * pipeline.compile(true);
 * }</pre>
 */
public class VilPipeline {

    private final String name;
    private int port;
    private String token = "shm";
    private final Map<String, Map<String, Object>> nodes = new LinkedHashMap<>();
    private final List<Map<String, String>> routes = new ArrayList<>();
    private final List<Map<String, Object>> semanticTypes = new ArrayList<>();
    private final List<Map<String, Object>> errors = new ArrayList<>();
    private Map<String, Object> state = null;
    private final List<Map<String, String>> failover = new ArrayList<>();
    private final List<Map<String, Object>> sseEvents = new ArrayList<>();
    private final List<Map<String, Object>> wsEvents = new ArrayList<>();

    // =========================================================================
    // Constructor
    // =========================================================================

    public VilPipeline(String name) {
        this(name, 3080);
    }

    public VilPipeline(String name, int port) {
        this.name = name;
        this.port = port;
    }

    // =========================================================================
    // Node builders
    // =========================================================================

    /**
     * Add an HttpSink node (webhook trigger endpoint).
     *
     * @param port TCP port.
     * @param path URL path.
     * @param nodeName Node name (default: http_sink).
     * @return this for chaining.
     */
    public VilPipeline sink(int port, String path, String nodeName) {
        if (nodeName == null) nodeName = "http_sink";
        if (path == null) path = "/trigger";
        Map<String, Object> node = new LinkedHashMap<>();
        node.put("type", "http_sink");
        node.put("port", port > 0 ? port : 3080);
        node.put("path", path);
        nodes.put(nodeName, node);
        this.port = port > 0 ? port : 3080;
        return this;
    }

    /** Add an HttpSink node with defaults. */
    public VilPipeline sink(int port, String path) {
        return sink(port, path, null);
    }

    /**
     * Add an HttpSource node (upstream inference endpoint).
     *
     * @param url Upstream URL.
     * @param format Response format (sse, json, raw).
     * @param jsonTap JSONPath-like expression.
     * @param nodeName Node name (default: http_source).
     * @param postBody Request body (Map or JSON string).
     * @return this for chaining.
     */
    public VilPipeline source(String url, String format, String jsonTap,
                               String nodeName, Object postBody) {
        if (nodeName == null) nodeName = "http_source";
        if (format == null) format = "sse";
        Map<String, Object> node = new LinkedHashMap<>();
        node.put("type", "http_source");
        node.put("url", url);
        node.put("format", format);
        if (jsonTap != null) node.put("json_tap", jsonTap);
        if (postBody != null) node.put("post_body", postBody);
        nodes.put(nodeName, node);
        return this;
    }

    /** Add an HttpSource node with minimal parameters. */
    public VilPipeline source(String url, String format, String jsonTap) {
        return source(url, format, jsonTap, null, null);
    }

    /**
     * Add a transform node with optional inline code.
     *
     * @param nodeName Node name.
     * @param fnBody Rust expression or handler body (nullable).
     * @return this for chaining.
     */
    public VilPipeline transform(String nodeName, String fnBody) {
        Map<String, Object> node = new LinkedHashMap<>();
        node.put("type", "transform");
        if (fnBody != null) {
            Map<String, String> code = new LinkedHashMap<>();
            code.put("mode", "expr");
            code.put("body", fnBody);
            node.put("code", code);
        }
        nodes.put(nodeName, node);
        return this;
    }

    /** Add a transform node without inline code. */
    public VilPipeline transform(String nodeName) {
        return transform(nodeName, null);
    }

    /**
     * Add a route between node ports.
     *
     * @param from Source port.
     * @param to Destination port.
     * @param mode Transfer mode (LoanWrite, Copy).
     * @return this for chaining.
     */
    public VilPipeline route(String from, String to, String mode) {
        if (mode == null) mode = "LoanWrite";
        Map<String, String> r = new LinkedHashMap<>();
        r.put("from", from);
        r.put("to", to);
        r.put("mode", mode);
        routes.add(r);
        return this;
    }

    // =========================================================================
    // Semantic type declarations
    // =========================================================================

    /** Declare a semantic type (state/event/fault/decision). */
    public VilPipeline semanticType(String name, String kind,
                                     Map<String, String> fields,
                                     List<String> variants) {
        Map<String, Object> st = new LinkedHashMap<>();
        st.put("name", name);
        st.put("kind", kind);
        if (fields != null) st.put("fields", fields);
        if (variants != null) st.put("variants", variants);
        semanticTypes.add(st);
        return this;
    }

    /** Declare a semantic state type AND set service state. */
    public VilPipeline state(String name, Map<String, String> fields) {
        semanticType(name, "state", fields, null);
        Map<String, Object> s = new LinkedHashMap<>();
        s.put("type", "private_heap");
        s.put("fields", fields);
        this.state = s;
        return this;
    }

    /** Shorthand for semanticType(kind='event'). */
    public VilPipeline event(String name, Map<String, String> fields) {
        return semanticType(name, "event", fields, null);
    }

    /** Shorthand for semanticType(kind='fault'). */
    public VilPipeline fault(String name, List<String> variants) {
        return semanticType(name, "fault", null, variants);
    }

    /** Shorthand for semanticType(kind='decision'). */
    public VilPipeline decision(String name, Map<String, String> fields) {
        return semanticType(name, "decision", fields, null);
    }

    /** Declare a VilError variant. */
    public VilPipeline error(String name, int status, String code,
                              Boolean retry, Map<String, String> fields) {
        Map<String, Object> err = new LinkedHashMap<>();
        err.put("name", name);
        err.put("status", status);
        if (code != null) err.put("code", code);
        if (retry != null) err.put("retry", retry);
        if (fields != null) err.put("fields", fields);
        errors.add(err);
        return this;
    }

    /** Declare a failover entry. */
    public VilPipeline failover(String primary, String backup, String strategy) {
        if (strategy == null) strategy = "immediate";
        Map<String, String> e = new LinkedHashMap<>();
        e.put("primary", primary);
        e.put("backup", backup);
        e.put("strategy", strategy);
        failover.add(e);
        return this;
    }

    /** Declare an SSE event type. */
    public VilPipeline sseEvent(String name, Map<String, String> fields, String topic) {
        Map<String, Object> ev = new LinkedHashMap<>();
        ev.put("name", name);
        if (topic != null) ev.put("topic", topic);
        ev.put("fields", fields);
        sseEvents.add(ev);
        return this;
    }

    /** Declare a WebSocket event type. */
    public VilPipeline wsEvent(String name, Map<String, String> fields, String topic) {
        Map<String, Object> ev = new LinkedHashMap<>();
        ev.put("name", name);
        if (topic != null) ev.put("topic", topic);
        ev.put("fields", fields);
        wsEvents.add(ev);
        return this;
    }

    // =========================================================================
    // YAML generation
    // =========================================================================

    /**
     * Generate YAML manifest string for {@code vil compile}.
     *
     * @return YAML string matching WorkflowManifest format.
     */
    public String toYaml() {
        StringBuilder sb = new StringBuilder();
        sb.append("vil_version: \"6.0.0\"\n");
        sb.append("name: ").append(name).append("\n");
        sb.append("port: ").append(port).append("\n");
        sb.append("token: ").append(token).append("\n");

        // Semantic types
        appendSemanticTypes(sb);
        appendErrors(sb);
        appendState(sb);
        appendFailover(sb);
        appendEvents(sb, sseEvents, "sse_events");
        appendEvents(sb, wsEvents, "ws_events");

        // Nodes
        if (!nodes.isEmpty()) {
            sb.append("\nnodes:\n");
            for (Map.Entry<String, Map<String, Object>> entry : nodes.entrySet()) {
                sb.append("  ").append(entry.getKey()).append(":\n");
                Map<String, Object> node = entry.getValue();
                sb.append("    type: ").append(node.get("type")).append("\n");
                if (node.get("port") != null) sb.append("    port: ").append(node.get("port")).append("\n");
                if (node.get("path") != null) sb.append("    path: \"").append(node.get("path")).append("\"\n");
                if (node.get("url") != null) sb.append("    url: \"").append(node.get("url")).append("\"\n");
                if (node.get("format") != null) sb.append("    format: ").append(node.get("format")).append("\n");
                if (node.get("json_tap") != null) sb.append("    json_tap: \"").append(node.get("json_tap")).append("\"\n");
                if (node.get("dialect") != null) sb.append("    dialect: ").append(node.get("dialect")).append("\n");
                if (node.get("post_body") != null) sb.append("    post_body: ").append(toJsonString(node.get("post_body"))).append("\n");
                if (node.get("code") instanceof Map) {
                    @SuppressWarnings("unchecked")
                    Map<String, String> code = (Map<String, String>) node.get("code");
                    sb.append("    code:\n");
                    sb.append("      mode: ").append(code.get("mode")).append("\n");
                    sb.append("      body: \"").append(code.get("body")).append("\"\n");
                }
            }
        }

        // Routes
        if (!routes.isEmpty()) {
            sb.append("\nroutes:\n");
            for (Map<String, String> r : routes) {
                sb.append("  - from: ").append(r.get("from")).append("\n");
                sb.append("    to: ").append(r.get("to")).append("\n");
                sb.append("    mode: ").append(r.get("mode")).append("\n");
            }
        }

        return sb.toString();
    }

    /** @deprecated Use {@link #toYaml()} instead. */
    @Deprecated
    public String toManifest() {
        return toYaml();
    }

    // =========================================================================
    // Compile
    // =========================================================================

    /**
     * Call {@code vil compile} with the generated YAML manifest.
     *
     * <p>In manifest mode ({@code VIL_COMPILE_MODE=manifest}), prints
     * YAML to stdout. Otherwise invokes the vil CLI compiler.
     *
     * @param release Build in release mode.
     */
    public void compile(boolean release) {
        if ("manifest".equals(System.getenv("VIL_COMPILE_MODE"))) {
            System.out.print(toYaml());
            return;
        }

        try {
            File tmp = File.createTempFile("vil-manifest-", ".yaml");
            tmp.deleteOnExit();
            try (FileWriter fw = new FileWriter(tmp)) {
                fw.write(toYaml());
            }

            List<String> cmd = new ArrayList<>();
            cmd.add("vil");
            cmd.add("compile");
            cmd.add("--manifest");
            cmd.add(tmp.getAbsolutePath());
            if (release) cmd.add("--release");
            cmd.add("--output");
            cmd.add(name);

            System.out.printf("  Compiling pipeline: %s%n", name);
            System.out.printf("  Command: %s%n", String.join(" ", cmd));

            ProcessBuilder pb = new ProcessBuilder(cmd);
            pb.inheritIO();
            Process proc = pb.start();
            int exitCode = proc.waitFor();
            if (exitCode != 0) {
                System.out.printf("  Compilation failed (exit code %d)%n", exitCode);
                System.out.printf("  Manifest: %s%n", tmp.getAbsolutePath());
            }
        } catch (Exception e) {
            System.out.println("  'vil' CLI not found or error: " + e.getMessage());
        }
    }

    /** @deprecated Use {@link #compile(boolean)} instead. */
    @Deprecated
    public void run() {
        compile(true);
    }

    // =========================================================================
    // YAML helpers (shared)
    // =========================================================================

    private void appendSemanticTypes(StringBuilder sb) {
        if (semanticTypes.isEmpty()) return;
        sb.append("semantic_types:\n");
        for (Map<String, Object> st : semanticTypes) {
            sb.append("  - name: ").append(st.get("name")).append("\n");
            sb.append("    kind: ").append(st.get("kind")).append("\n");
            if (st.get("fields") instanceof Map) {
                sb.append("    fields:\n");
                @SuppressWarnings("unchecked")
                Map<String, String> fields = (Map<String, String>) st.get("fields");
                for (Map.Entry<String, String> f : fields.entrySet()) {
                    sb.append("      - name: ").append(f.getKey()).append("\n");
                    sb.append("        type: ").append(f.getValue()).append("\n");
                }
            }
            if (st.get("variants") instanceof List) {
                sb.append("    variants:\n");
                @SuppressWarnings("unchecked")
                List<String> variants = (List<String>) st.get("variants");
                for (String v : variants) {
                    sb.append("      - ").append(v).append("\n");
                }
            }
        }
    }

    private void appendErrors(StringBuilder sb) {
        if (errors.isEmpty()) return;
        sb.append("errors:\n");
        for (Map<String, Object> err : errors) {
            sb.append("  - name: ").append(err.get("name")).append("\n");
            sb.append("    status: ").append(err.get("status")).append("\n");
            if (err.get("code") != null) sb.append("    code: ").append(err.get("code")).append("\n");
            if (err.get("retry") != null) sb.append("    retry: ").append(err.get("retry")).append("\n");
            if (err.get("fields") instanceof Map) {
                sb.append("    fields:\n");
                @SuppressWarnings("unchecked")
                Map<String, String> fields = (Map<String, String>) err.get("fields");
                for (Map.Entry<String, String> f : fields.entrySet()) {
                    sb.append("      - name: ").append(f.getKey()).append("\n");
                    sb.append("        type: ").append(f.getValue()).append("\n");
                }
            }
        }
    }

    private void appendState(StringBuilder sb) {
        if (state == null) return;
        sb.append("state:\n");
        sb.append("  type: ").append(state.get("type")).append("\n");
        sb.append("  fields:\n");
        @SuppressWarnings("unchecked")
        Map<String, String> stFields = (Map<String, String>) state.get("fields");
        for (Map.Entry<String, String> f : stFields.entrySet()) {
            sb.append("    - name: ").append(f.getKey()).append("\n");
            sb.append("      type: ").append(f.getValue()).append("\n");
        }
    }

    private void appendFailover(StringBuilder sb) {
        if (failover.isEmpty()) return;
        sb.append("failover:\n");
        sb.append("  entries:\n");
        for (Map<String, String> e : failover) {
            sb.append("    - primary: ").append(e.get("primary")).append("\n");
            sb.append("      backup: ").append(e.get("backup")).append("\n");
            sb.append("      strategy: ").append(e.get("strategy")).append("\n");
        }
    }

    private void appendEvents(StringBuilder sb, List<Map<String, Object>> events, String section) {
        if (events.isEmpty()) return;
        sb.append(section).append(":\n");
        for (Map<String, Object> ev : events) {
            sb.append("  - name: ").append(ev.get("name")).append("\n");
            if (ev.get("topic") != null) sb.append("    topic: ").append(ev.get("topic")).append("\n");
            sb.append("    fields:\n");
            @SuppressWarnings("unchecked")
            Map<String, String> evFields = (Map<String, String>) ev.get("fields");
            for (Map.Entry<String, String> f : evFields.entrySet()) {
                sb.append("      - name: ").append(f.getKey()).append("\n");
                sb.append("        type: ").append(f.getValue()).append("\n");
            }
        }
    }

    // =========================================================================
    // JSON helpers
    // =========================================================================

    @SuppressWarnings("unchecked")
    static String toJsonString(Object obj) {
        if (obj instanceof String) return (String) obj;
        if (obj instanceof Map) return mapToJson((Map<String, Object>) obj);
        return String.valueOf(obj);
    }

    @SuppressWarnings("unchecked")
    static String mapToJson(Map<String, Object> map) {
        StringBuilder sb = new StringBuilder("{");
        boolean first = true;
        for (Map.Entry<String, Object> e : map.entrySet()) {
            if (!first) sb.append(",");
            first = false;
            sb.append("\"").append(e.getKey()).append("\":");
            sb.append(valueToJson(e.getValue()));
        }
        sb.append("}");
        return sb.toString();
    }

    @SuppressWarnings("unchecked")
    static String valueToJson(Object v) {
        if (v == null) return "null";
        if (v instanceof String) return "\"" + ((String) v).replace("\"", "\\\"") + "\"";
        if (v instanceof Number || v instanceof Boolean) return String.valueOf(v);
        if (v instanceof Map) return mapToJson((Map<String, Object>) v);
        if (v instanceof List) {
            StringBuilder sb = new StringBuilder("[");
            boolean first = true;
            for (Object item : (List<?>) v) {
                if (!first) sb.append(",");
                first = false;
                sb.append(valueToJson(item));
            }
            sb.append("]");
            return sb.toString();
        }
        return "\"" + String.valueOf(v).replace("\"", "\\\"") + "\"";
    }

    // =========================================================================
    // Getters
    // =========================================================================

    public String getName() { return name; }
    public int getPort() { return port; }
    public String getToken() { return token; }
    public void setToken(String token) { this.token = token; }
}
