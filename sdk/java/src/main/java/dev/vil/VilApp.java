package dev.vil;

import java.io.File;
import java.io.FileWriter;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * VilApp — VX process-oriented application builder.
 *
 * <p>Composes ServiceProcess instances into a single application
 * manifest. {@code vil compile} transpiles it to a native binary.
 *
 * <p>No FFI, no JNI — pure YAML generation.
 *
 * <p>Example:
 * <pre>{@code
 * ServiceProcess chatSvc = new ServiceProcess("chat")
 *     .prefix("/api/chat")
 *     .endpoint("POST", "/query", "query_handler")
 *     .emits("ChatEvent")
 *     .faults("ChatFault")
 *     .manages("ChatState");
 *
 * new VilApp("my-ai-gateway")
 *     .port(8080)
 *     .service(chatSvc)
 *     .compile(true);
 * }</pre>
 */
public class VilApp {

    private final String name;
    private int port = 8080;
    private final List<ServiceProcess> services = new ArrayList<>();
    private final List<Map<String, Object>> semanticTypes = new ArrayList<>();

    public VilApp(String name) {
        this.name = name;
    }

    /** Set listening port. */
    public VilApp port(int port) {
        this.port = port;
        return this;
    }

    /** Add a ServiceProcess. */
    public VilApp service(ServiceProcess svc) {
        this.services.add(svc);
        return this;
    }

    /** Create and add a new ServiceProcess by name. */
    public ServiceProcess service(String name) {
        ServiceProcess svc = new ServiceProcess(name);
        this.services.add(svc);
        return svc;
    }

    /** Declare a semantic type at the app level. */
    public VilApp semanticType(String name, String kind,
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

    /** Shorthand: declare a semantic state type. */
    public VilApp state(String name, Map<String, String> fields) {
        return semanticType(name, "state", fields, null);
    }

    /** Shorthand: declare a semantic event type. */
    public VilApp event(String name, Map<String, String> fields) {
        return semanticType(name, "event", fields, null);
    }

    /** Shorthand: declare a semantic fault type. */
    public VilApp fault(String name, List<String> variants) {
        return semanticType(name, "fault", null, variants);
    }

    // =========================================================================
    // Getters
    // =========================================================================

    public String getName() { return name; }
    public int getPort() { return port; }
    public List<ServiceProcess> getServices() { return services; }

    // =========================================================================
    // YAML generation
    // =========================================================================

    /**
     * Generate YAML manifest string for {@code vil compile}.
     *
     * @return YAML string matching WorkflowManifest format (vil_app mode).
     */
    public String toYaml() {
        StringBuilder sb = new StringBuilder();
        sb.append("vil_version: \"6.0.0\"\n");
        sb.append("name: ").append(name).append("\n");
        sb.append("port: ").append(port).append("\n");
        sb.append("mode: vil_app\n");

        // Semantic types
        if (!semanticTypes.isEmpty()) {
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

        sb.append("\nservices:\n");
        for (ServiceProcess svc : services) {
            sb.append("  - name: ").append(svc.getName()).append("\n");
            sb.append("    prefix: \"").append(svc.getPrefix()).append("\"\n");
            if (svc.getEmits() != null) sb.append("    emits: ").append(svc.getEmits()).append("\n");
            if (svc.getFaults() != null) sb.append("    faults: ").append(svc.getFaults()).append("\n");
            if (svc.getManages() != null) sb.append("    manages: ").append(svc.getManages()).append("\n");

            List<ServiceProcess.EndpointDef> endpoints = svc.getEndpoints();
            if (!endpoints.isEmpty()) {
                sb.append("    endpoints:\n");
                for (ServiceProcess.EndpointDef ep : endpoints) {
                    sb.append("      - method: ").append(ep.method).append("\n");
                    sb.append("        path: \"").append(ep.path).append("\"\n");
                    sb.append("        handler: ").append(ep.handler).append("\n");
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

            System.out.printf("  Compiling app: %s%n", name);
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
}
