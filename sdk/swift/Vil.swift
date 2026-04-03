// =============================================================================
// VIL SDK for Swift — Single-file, zero-dependency SDK.
//
// Usage:
//   swift main.swift                                  — compiles via `vil compile`
//   VIL_COMPILE_MODE=manifest swift main.swift        — prints YAML manifest
//
// API:
//   let p = VilPipeline("name", port: 3080)
//       .sink("webhook", port: 3080, path: "/trigger")
//       .source("inference", url: "http://...", format: "sse", jsonTap: "choices[0].delta.content")
//       .route("webhook.trigger_out", to: "inference.trigger_in", mode: "LoanWrite")
//   p.compile()
// =============================================================================

import Foundation

class ServiceProcess {
    let name: String
    let prefix: String
    struct Ep { let method: String; let path: String; let handler: String }
    var endpoints: [Ep] = []

    init(_ name: String, prefix: String? = nil) {
        self.name = name
        self.prefix = prefix ?? "/api/\(name)"
    }

    @discardableResult
    func endpoint(_ method: String, _ path: String, handler: String? = nil) -> ServiceProcess {
        let h = handler ?? "\(method.lowercased())_\(path.trimmingCharacters(in: CharacterSet(charactersIn: "/")).replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: ":", with: ""))"
        endpoints.append(Ep(method: method, path: path, handler: h))
        return self
    }
}

class VilPipeline {
    private let name: String
    private var port: Int
    private struct Node { let type_: String; let port: Int; let path: String?; let url: String?; let format: String?; let jsonTap: String?; let dialect: String? }
    private struct Route { let from: String; let to: String; let mode: String }
    private var nodeOrder: [String] = []
    private var nodes: [String: Node] = [:]
    private var routes: [Route] = []

    init(_ name: String, port: Int) { self.name = name; self.port = port }

    @discardableResult
    func sink(_ nodeName: String, port: Int = 3080, path: String = "/trigger") -> VilPipeline {
        nodes[nodeName] = Node(type_: "http_sink", port: port, path: path, url: nil, format: nil, jsonTap: nil, dialect: nil)
        if !nodeOrder.contains(nodeName) { nodeOrder.append(nodeName) }
        self.port = port
        return self
    }

    @discardableResult
    func source(_ nodeName: String, url: String, format: String = "sse",
                jsonTap: String? = nil, dialect: String? = nil) -> VilPipeline {
        nodes[nodeName] = Node(type_: "http_source", port: 0, path: nil, url: url, format: format, jsonTap: jsonTap, dialect: dialect)
        if !nodeOrder.contains(nodeName) { nodeOrder.append(nodeName) }
        return self
    }

    @discardableResult
    func route(_ from: String, to: String, mode: String) -> VilPipeline {
        routes.append(Route(from: from, to: to, mode: mode))
        return self
    }

    func toYaml() -> String {
        var lines: [String] = []
        lines.append("vil_version: \"6.0.0\"")
        lines.append("name: \(name)")
        lines.append("port: \(port)")
        lines.append("token: shm")
        if !nodeOrder.isEmpty {
            lines.append(""); lines.append("nodes:")
            for n in nodeOrder {
                let nd = nodes[n]!
                lines.append("  \(n):")
                lines.append("    type: \(nd.type_)")
                if nd.port != 0 { lines.append("    port: \(nd.port)") }
                if let p = nd.path { lines.append("    path: \"\(p)\"") }
                if let u = nd.url { lines.append("    url: \"\(u)\"") }
                if let f = nd.format { lines.append("    format: \(f)") }
                if let j = nd.jsonTap { lines.append("    json_tap: \"\(j)\"") }
                if let d = nd.dialect { lines.append("    dialect: \(d)") }
            }
        }
        if !routes.isEmpty {
            lines.append(""); lines.append("routes:")
            for r in routes {
                lines.append("  - from: \(r.from)")
                lines.append("    to: \(r.to)")
                lines.append("    mode: \(r.mode)")
            }
        }
        return lines.joined(separator: "\n") + "\n"
    }

    func compile() {
        let yaml = toYaml()
        if ProcessInfo.processInfo.environment["VIL_COMPILE_MODE"] == "manifest" {
            print(yaml, terminator: ""); return
        }
        print("  Compiling: \(name)")
        print(yaml, terminator: "")
    }
}

class VilServer {
    private let name: String
    private let port: Int
    private var services: [ServiceProcess] = []
    private struct SemType { let name: String; let kind: String }
    private var semanticTypes: [SemType] = []

    init(_ name: String, port: Int) { self.name = name; self.port = port }

    @discardableResult
    func service(_ svc: ServiceProcess) -> VilServer { services.append(svc); return self }

    @discardableResult
    func semanticType(_ name: String, kind: String) -> VilServer { semanticTypes.append(SemType(name: name, kind: kind)); return self }

    func toYaml() -> String {
        var lines: [String] = []
        lines.append("vil_version: \"6.0.0\"")
        lines.append("name: \(name)")
        lines.append("port: \(port)")
        if !semanticTypes.isEmpty {
            lines.append("semantic_types:")
            for st in semanticTypes { lines.append("  - name: \(st.name)"); lines.append("    kind: \(st.kind)") }
        }
        if !services.isEmpty {
            lines.append("services:")
            for svc in services {
                lines.append("  - name: \(svc.name)")
                lines.append("    prefix: \"\(svc.prefix)\"")
                if !svc.endpoints.isEmpty {
                    lines.append("    endpoints:")
                    for ep in svc.endpoints {
                        lines.append("      - method: \(ep.method)")
                        lines.append("        path: \"\(ep.path)\"")
                        lines.append("        handler: \(ep.handler)")
                    }
                }
            }
        }
        return lines.joined(separator: "\n") + "\n"
    }

    func compile() {
        let yaml = toYaml()
        if ProcessInfo.processInfo.environment["VIL_COMPILE_MODE"] == "manifest" {
            print(yaml, terminator: ""); return
        }
        print("  Compiling: \(name)")
        print(yaml, terminator: "")
    }
}
