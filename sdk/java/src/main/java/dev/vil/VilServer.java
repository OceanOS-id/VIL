package dev.vil;

import java.io.File;
import java.io.FileWriter;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * VIL Server — Server DSL -> YAML manifest -> native binary with VIL Way.
 *
 * <p>Generates a YAML manifest with endpoints: section. {@code vil compile}
 * transpiles it to Rust code using VIL Way patterns:
 * <ul>
 *   <li>{@code ctx: ServiceCtx} (not Extension&lt;T&gt;)</li>
 *   <li>{@code body: ShmSlice} (not Json&lt;T&gt;)</li>
 *   <li>{@code .state(x)} (not .extension(x))</li>
 *   <li>{@code body.json::<T>()} for deserialization</li>
 * </ul>
 *
 * <p>No FFI, no JNI — pure YAML generation.
 *
 * <p>Example:
 * <pre>{@code
 * VilServer app = new VilServer("hello", 8080);
 * app.get("/greet/:name", null, VilServer.output("message", "String"), null);
 * app.post("/chat", VilServer.input("prompt", "String"),
 *     null, VilServer.sse("http://localhost:4545/v1/chat", null));
 * app.compile(true);
 * }</pre>
 */
public class VilServer {

    private final String name;
    private final int port;
    private final List<ServiceProcess> services = new ArrayList<>();
    private final List<EndpointMeta> endpoints = new ArrayList<>();
    private final List<Map<String, Object>> semanticTypes = new ArrayList<>();
    private final List<Map<String, Object>> errors = new ArrayList<>();
    private Map<String, Object> state = null;
    private Map<String, Object> mesh = null;
    private final List<Map<String, String>> failover = new ArrayList<>();
    private final List<Map<String, Object>> sseEvents = new ArrayList<>();
    private final List<Map<String, Object>> wsEvents = new ArrayList<>();

    // =========================================================================
    // Constructor
    // =========================================================================

    public VilServer(String name, int port) {
        this.name = name;
        this.port = port;
    }

    public VilServer(String name) {
        this(name, 8080);
    }

    // =========================================================================
    // HTTP method registration
    // =========================================================================

    /** Register a GET endpoint. */
    public VilServer get(String path, Map<String, FieldSpec> input,
                          Map<String, FieldSpec> output, UpstreamSpec upstream) {
        return addEndpoint("GET", path, input, output, upstream, null, null);
    }

    /** Register a GET endpoint with handler name. */
    public VilServer get(String path, Map<String, FieldSpec> input,
                          Map<String, FieldSpec> output, UpstreamSpec upstream,
                          String handler) {
        return addEndpoint("GET", path, input, output, upstream, handler, null);
    }

    /** Register a POST endpoint. */
    public VilServer post(String path, Map<String, FieldSpec> input,
                           Map<String, FieldSpec> output, UpstreamSpec upstream) {
        return addEndpoint("POST", path, input, output, upstream, null, null);
    }

    /** Register a POST endpoint with handler name. */
    public VilServer post(String path, Map<String, FieldSpec> input,
                           Map<String, FieldSpec> output, UpstreamSpec upstream,
                           String handler) {
        return addEndpoint("POST", path, input, output, upstream, handler, null);
    }

    /** Register a PUT endpoint. */
    public VilServer put(String path, Map<String, FieldSpec> input,
                          Map<String, FieldSpec> output) {
        return addEndpoint("PUT", path, input, output, null, null, null);
    }

    /** Register a DELETE endpoint. */
    public VilServer delete(String path) {
        return addEndpoint("DELETE", path, null, null, null, null, null);
    }

    /** Register a DELETE endpoint with handler name. */
    public VilServer delete(String path, String handler) {
        return addEndpoint("DELETE", path, null, null, null, handler, null);
    }

    private VilServer addEndpoint(String method, String path,
                                   Map<String, FieldSpec> input,
                                   Map<String, FieldSpec> output,
                                   UpstreamSpec upstream,
                                   String handler, String execClass) {
        if (handler == null) {
            String slug = path.replaceAll("^/+", "").replace("/", "_").replace(":", "");
            if (slug.isEmpty()) slug = "index";
            handler = method.toLowerCase() + "_" + slug;
        }
        EndpointMeta ep = new EndpointMeta();
        ep.method = method;
        ep.path = path;
        ep.handlerName = handler;
        ep.input = input;
        ep.output = output;
        ep.upstream = upstream;
        ep.execClass = execClass;
        endpoints.add(ep);
        return this;
    }

    // =========================================================================
    // Semantic type declarations
    // =========================================================================

    /** Declare a semantic type (state/event/fault/decision). */
    public VilServer semanticType(String name, String kind,
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
    public VilServer state(String name, Map<String, String> fields) {
        semanticType(name, "state", fields, null);
        Map<String, Object> s = new LinkedHashMap<>();
        s.put("type", "private_heap");
        s.put("fields", fields);
        this.state = s;
        return this;
    }

    /** Shorthand for semanticType(kind='event'). */
    public VilServer event(String name, Map<String, String> fields) {
        return semanticType(name, "event", fields, null);
    }

    /** Shorthand for semanticType(kind='fault'). */
    public VilServer fault(String name, List<String> variants) {
        return semanticType(name, "fault", null, variants);
    }

    /** Shorthand for semanticType(kind='decision'). */
    public VilServer decision(String name, Map<String, String> fields) {
        return semanticType(name, "decision", fields, null);
    }

    /** Declare a VilError variant. */
    public VilServer error(String name, int status, String code,
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

    // =========================================================================
    // Mesh / Failover
    // =========================================================================

    /** Declare Tri-Lane mesh routes. */
    @SuppressWarnings("unchecked")
    public VilServer mesh(List<Map<String, String>> routes) {
        Map<String, Object> m = new LinkedHashMap<>();
        m.put("routes", routes);
        this.mesh = m;
        return this;
    }

    /** Declare a failover entry. */
    public VilServer failover(String primary, String backup, String strategy) {
        if (strategy == null) strategy = "immediate";
        Map<String, String> e = new LinkedHashMap<>();
        e.put("primary", primary);
        e.put("backup", backup);
        e.put("strategy", strategy);
        failover.add(e);
        return this;
    }

    // =========================================================================
    // Event declarations
    // =========================================================================

    /** Declare an SSE event type. */
    public VilServer sseEvent(String name, Map<String, String> fields, String topic) {
        Map<String, Object> ev = new LinkedHashMap<>();
        ev.put("name", name);
        if (topic != null) ev.put("topic", topic);
        ev.put("fields", fields);
        sseEvents.add(ev);
        return this;
    }

    /** Declare a WebSocket event type. */
    public VilServer wsEvent(String name, Map<String, String> fields, String topic) {
        Map<String, Object> ev = new LinkedHashMap<>();
        ev.put("name", name);
        if (topic != null) ev.put("topic", topic);
        ev.put("fields", fields);
        wsEvents.add(ev);
        return this;
    }

    // =========================================================================
    // ServiceProcess composition
    // =========================================================================

    /**
     * Create and register a VX ServiceProcess.
     *
     * @param name Service name.
     * @param prefix URL prefix (default: /api/{name}).
     * @return The new ServiceProcess.
     */
    public ServiceProcess serviceProcess(String name, String prefix) {
        ServiceProcess svc = new ServiceProcess(name);
        if (prefix != null) svc.prefix(prefix);
        services.add(svc);
        return svc;
    }

    /** Create a ServiceProcess with default prefix. */
    public ServiceProcess serviceProcess(String name) {
        return serviceProcess(name, null);
    }

    /** Add an existing ServiceProcess. */
    public VilServer service(ServiceProcess svc) {
        services.add(svc);
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

        // Semantic types
        appendSemanticTypes(sb);
        appendErrors(sb);
        appendState(sb);
        appendMesh(sb);
        appendFailover(sb);
        appendEvents(sb, sseEvents, "sse_events");
        appendEvents(sb, wsEvents, "ws_events");

        // Endpoints (server mode)
        if (!endpoints.isEmpty()) {
            sb.append("endpoints:\n");
            for (EndpointMeta ep : endpoints) {
                sb.append("  - method: ").append(ep.method).append("\n");
                sb.append("    path: \"").append(ep.path).append("\"\n");
                sb.append("    handler: ").append(ep.handlerName).append("\n");
                if (ep.execClass != null) sb.append("    exec_class: ").append(ep.execClass).append("\n");
                if (ep.input != null && !ep.input.isEmpty()) {
                    sb.append("    input:\n");
                    sb.append("      type: json\n");
                    sb.append("      fields:\n");
                    for (Map.Entry<String, FieldSpec> e : ep.input.entrySet()) {
                        sb.append("        - name: ").append(e.getKey()).append("\n");
                        sb.append("          type: ").append(e.getValue().type).append("\n");
                        if (e.getValue().required) sb.append("          required: true\n");
                        if (e.getValue().itemsType != null) sb.append("          items_type: ").append(e.getValue().itemsType).append("\n");
                    }
                }
                if (ep.output != null && !ep.output.isEmpty()) {
                    sb.append("    output:\n");
                    sb.append("      type: json\n");
                    sb.append("      fields:\n");
                    for (Map.Entry<String, FieldSpec> e : ep.output.entrySet()) {
                        sb.append("        - name: ").append(e.getKey()).append("\n");
                        sb.append("          type: ").append(e.getValue().type).append("\n");
                        if (e.getValue().required) sb.append("          required: true\n");
                        if (e.getValue().itemsType != null) sb.append("          items_type: ").append(e.getValue().itemsType).append("\n");
                    }
                }
                if (ep.upstream != null) {
                    sb.append("    upstream:\n");
                    sb.append("      type: ").append(ep.upstream.type).append("\n");
                    sb.append("      url: \"").append(ep.upstream.url).append("\"\n");
                    if (ep.upstream.method != null) sb.append("      method: ").append(ep.upstream.method).append("\n");
                    if (ep.upstream.bodyTemplate != null) sb.append("      body_template: ").append(ep.upstream.bodyTemplate).append("\n");
                }
            }
        }

        // Services (VX app mode)
        if (!services.isEmpty()) {
            sb.append("services:\n");
            for (ServiceProcess svc : services) {
                sb.append("  - name: ").append(svc.getName()).append("\n");
                sb.append("    prefix: \"").append(svc.getPrefix()).append("\"\n");
                if (svc.getEmits() != null) sb.append("    emits: ").append(svc.getEmits()).append("\n");
                if (svc.getFaults() != null) sb.append("    faults: ").append(svc.getFaults()).append("\n");
                if (svc.getManages() != null) sb.append("    manages: ").append(svc.getManages()).append("\n");
                List<ServiceProcess.EndpointDef> svcEndpoints = svc.getEndpoints();
                if (!svcEndpoints.isEmpty()) {
                    sb.append("    endpoints:\n");
                    for (ServiceProcess.EndpointDef ep : svcEndpoints) {
                        sb.append("      - method: ").append(ep.method).append("\n");
                        sb.append("        path: \"").append(ep.path).append("\"\n");
                        sb.append("        handler: ").append(ep.handler).append("\n");
                    }
                }
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

            System.out.printf("  Compiling server: %s%n", name);
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
    // YAML helpers
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

    private void appendMesh(StringBuilder sb) {
        if (mesh == null) return;
        sb.append("mesh:\n");
        sb.append("  routes:\n");
        @SuppressWarnings("unchecked")
        List<Map<String, String>> meshRoutes = (List<Map<String, String>>) mesh.get("routes");
        for (Map<String, String> r : meshRoutes) {
            sb.append("    - from: ").append(r.get("from")).append("\n");
            sb.append("      to: ").append(r.get("to")).append("\n");
            sb.append("      lane: ").append(r.get("lane")).append("\n");
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
    // DSL field/upstream helpers (static)
    // =========================================================================

    /** DSL field specification. */
    public static class FieldSpec {
        public final String type;
        public final boolean required;
        public final String itemsType;

        public FieldSpec(String type, boolean required, String itemsType) {
            this.type = type;
            this.required = required;
            this.itemsType = itemsType;
        }
    }

    /** Declare a String field. */
    public static FieldSpec string_() { return new FieldSpec("String", false, null); }
    /** Declare a required String field. */
    public static FieldSpec stringRequired() { return new FieldSpec("String", true, null); }
    /** Declare a u64 field. */
    public static FieldSpec number_() { return new FieldSpec("u64", false, null); }
    /** Declare a bool field. */
    public static FieldSpec boolean_() { return new FieldSpec("bool", false, null); }
    /** Declare a Vec field. */
    public static FieldSpec array(String itemsType) { return new FieldSpec("Vec<" + itemsType + ">", false, itemsType); }

    /** Create an input schema with a single field (convenience). */
    public static Map<String, FieldSpec> input(String name, String type) {
        Map<String, FieldSpec> m = new LinkedHashMap<>();
        m.put(name, new FieldSpec(type, true, null));
        return m;
    }

    /** Create an output schema with a single field (convenience). */
    public static Map<String, FieldSpec> output(String name, String type) {
        Map<String, FieldSpec> m = new LinkedHashMap<>();
        m.put(name, new FieldSpec(type, false, null));
        return m;
    }

    /** DSL upstream declaration. */
    public static class UpstreamSpec {
        public final String type;
        public final String url;
        public final String method;
        public final String bodyTemplate;

        public UpstreamSpec(String type, String url, String method, String bodyTemplate) {
            this.type = type;
            this.url = url;
            this.method = method;
            this.bodyTemplate = bodyTemplate;
        }
    }

    /** Create an SSE upstream. */
    public static UpstreamSpec sse(String url, String bodyTemplate) {
        return new UpstreamSpec("sse", url, null, bodyTemplate);
    }

    /** Create an HTTP upstream. */
    public static UpstreamSpec http(String url, String method, String bodyTemplate) {
        return new UpstreamSpec("http", url, method, bodyTemplate);
    }

    /** Endpoint metadata container. */
    public static class EndpointMeta {
        public String method;
        public String path;
        public String handlerName;
        public Map<String, FieldSpec> input;
        public Map<String, FieldSpec> output;
        public UpstreamSpec upstream;
        public String execClass;
    }

    // =========================================================================
    // Getters
    // =========================================================================

    public String getName() { return name; }
    public int getPort() { return port; }
}
