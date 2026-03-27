package dev.vil;

import java.util.ArrayList;
import java.util.List;

/**
 * ServiceProcess — VX service builder with endpoints and semantic contracts.
 *
 * <p>Defines a named service with endpoints and semantic contract
 * declarations (emits, faults, manages/state). Used by VilServer
 * and VilApp to compose VX applications.
 *
 * <p>Example:
 * <pre>{@code
 * ServiceProcess svc = new ServiceProcess("rag-query")
 *     .prefix("/api/rag")
 *     .endpoint("POST", "/query", "query_handler")
 *     .endpoint("GET", "/stats", "stats_handler")
 *     .state("RagIndexState")
 *     .emits("RagQueryEvent")
 *     .faults("RagFault");
 * }</pre>
 */
public class ServiceProcess {

    private final String name;
    private String prefix;
    private String emits;
    private String faults;
    private String manages;
    private final List<EndpointDef> endpoints = new ArrayList<>();

    /** Endpoint definition. */
    public static class EndpointDef {
        public final String method;
        public final String path;
        public final String handler;

        public EndpointDef(String method, String path, String handler) {
            this.method = method;
            this.path = path;
            this.handler = handler;
        }
    }

    public ServiceProcess(String name) {
        this.name = name;
        this.prefix = "/api/" + name;
    }

    /** Set URL prefix for all endpoints. */
    public ServiceProcess prefix(String prefix) {
        this.prefix = prefix;
        return this;
    }

    /** Add an endpoint. */
    public ServiceProcess endpoint(String method, String path, String handler) {
        endpoints.add(new EndpointDef(method, path, handler));
        return this;
    }

    /**
     * Declare managed state type (semantic contract).
     * Maps to .state() in VIL Way generated code.
     */
    public ServiceProcess state(String typeName) {
        this.manages = typeName;
        return this;
    }

    /** Declare emitted event type (semantic contract). */
    public ServiceProcess emits(String typeName) {
        this.emits = typeName;
        return this;
    }

    /** Declare fault type (semantic contract). */
    public ServiceProcess faults(String typeName) {
        this.faults = typeName;
        return this;
    }

    /** @deprecated Use {@link #state(String)} instead. */
    @Deprecated
    public ServiceProcess manages(String typeName) {
        this.manages = typeName;
        return this;
    }

    // Getters
    public String getName() { return name; }
    public String getPrefix() { return prefix; }
    public String getEmits() { return emits; }
    public String getFaults() { return faults; }
    public String getManages() { return manages; }
    public List<EndpointDef> getEndpoints() { return endpoints; }
}
