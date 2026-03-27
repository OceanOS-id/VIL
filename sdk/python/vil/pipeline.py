"""VIL Transpile SDK — Write pipelines/servers in Python, compile to native Rust binary.

Pure YAML manifest generation. NO FFI, NO ctypes, NO runtime.
The generated YAML is consumed by `vil compile` which calls codegen.rs
to produce VIL Way Rust code (ServiceCtx, ShmSlice, .state()).

Usage:
    from vil import VilPipeline, VilServer

    pipeline = VilPipeline("ai-gateway")
    pipeline.sink(port=3080, path="/trigger")
    pipeline.source(url="http://localhost:4545/v1/chat", format="sse")
    yaml_str = pipeline.to_yaml()
    # vil compile --from python --input gateway.py --release
"""

import json
import os
import subprocess
import sys


# =============================================================================
# Field type helpers — used in schema declarations
# =============================================================================


def string(required=False):
    """Declare a String field."""
    return {"type": "String", "required": required}


def number(required=False):
    """Declare a u64 field."""
    return {"type": "u64", "required": required}


def boolean(required=False):
    """Declare a bool field."""
    return {"type": "bool", "required": required}


def array(items="string"):
    """Declare a Vec<T> field."""
    return {"type": f"Vec<{items}>", "required": False}


def sse(url, body=None):
    """Declare an SSE upstream connection."""
    result = {"type": "sse", "url": url}
    if body:
        result["body_template"] = body
    return result


def http(url, method="POST", body=None):
    """Declare an HTTP upstream connection."""
    result = {"type": "http", "url": url, "method": method}
    if body:
        result["body_template"] = body
    return result


# =============================================================================
# YAML emitter helpers (no PyYAML dependency)
# =============================================================================


def _yaml_fields(fields, indent=6):
    """Emit a list of field dicts as YAML."""
    prefix = " " * indent
    lines = []
    for f in fields:
        lines.append(f"{prefix}- name: {f['name']}")
        lines.append(f"{prefix}  type: {f['type']}")
        if f.get("required"):
            lines.append(f"{prefix}  required: true")
        if f.get("items_type"):
            lines.append(f"{prefix}  items_type: {f['items_type']}")
    return lines


def _yaml_semantic_types(semantic_types):
    """Emit semantic_types section."""
    if not semantic_types:
        return []
    lines = ["semantic_types:"]
    for st in semantic_types:
        lines.append(f"  - name: {st['name']}")
        lines.append(f"    kind: {st['kind']}")
        if st.get("fields"):
            lines.append("    fields:")
            lines.extend(_yaml_fields(st["fields"], indent=6))
        if st.get("variants"):
            lines.append("    variants:")
            for v in st["variants"]:
                lines.append(f"      - {v}")
    return lines


def _yaml_errors(errors):
    """Emit errors section."""
    if not errors:
        return []
    lines = ["errors:"]
    for err in errors:
        lines.append(f"  - name: {err['name']}")
        lines.append(f"    status: {err['status']}")
        if err.get("code"):
            lines.append(f"    code: {err['code']}")
        if err.get("retry") is not None:
            lines.append(f"    retry: {'true' if err['retry'] else 'false'}")
        if err.get("fields"):
            lines.append("    fields:")
            lines.extend(_yaml_fields(err["fields"], indent=6))
    return lines


def _yaml_state(state):
    """Emit state section."""
    if not state:
        return []
    lines = ["state:"]
    lines.append(f"  type: {state['type']}")
    lines.append("  fields:")
    lines.extend(_yaml_fields(state["fields"], indent=4))
    return lines


def _yaml_failover(failover_list):
    """Emit failover section."""
    if not failover_list:
        return []
    lines = ["failover:", "  entries:"]
    for e in failover_list:
        lines.append(f"    - primary: {e['primary']}")
        lines.append(f"      backup: {e['backup']}")
        lines.append(f"      strategy: {e['strategy']}")
    return lines


def _yaml_events(events, section_name):
    """Emit sse_events or ws_events section."""
    if not events:
        return []
    lines = [f"{section_name}:"]
    for ev in events:
        lines.append(f"  - name: {ev['name']}")
        if ev.get("topic"):
            lines.append(f"    topic: {ev['topic']}")
        lines.append("    fields:")
        lines.extend(_yaml_fields(ev["fields"], indent=6))
    return lines


def _build_schema(schema_dict):
    """Convert a dict of DSL field specs into a normalized schema."""
    if schema_dict is None:
        return None
    fields = []
    for name, spec in schema_dict.items():
        if isinstance(spec, dict):
            fields.append({"name": name, **spec})
        else:
            fields.append({"name": name, "type": "String"})
    return {"type": "json", "fields": fields}


def _make_semantic_entry(name, kind, fields=None, variants=None):
    """Build a semantic type dict."""
    return {
        "name": name,
        "kind": kind,
        "fields": [{"name": n, "type": t} for n, t in (fields or {}).items()],
        "variants": variants or [],
    }


# =============================================================================
# VilPipeline — SSE Pipeline DSL (HttpSink + HttpSource + Tri-Lane)
# =============================================================================


class VilPipeline:
    """Declarative pipeline DSL -> YAML manifest -> native binary.

    Generates a YAML manifest with pipeline: section (nodes + routes).
    ``vil compile`` transpiles it to a native Rust binary using VIL Way
    patterns (ServiceCtx, ShmSlice).

    Example::

        pipeline = VilPipeline("ai-gateway")
        pipeline.sink(port=3080, path="/trigger")
        pipeline.source(url="http://localhost:4545/v1/chat/completions",
                        format="sse")
        pipeline.compile()
    """

    def __init__(self, name, port=3080):
        self.name = name
        self.port = port
        self.token = "shm"
        self._nodes = {}
        self._routes = []
        self._semantic_types = []
        self._errors = []
        self._state = None
        self._failover = []
        self._sse_events = []
        self._ws_events = []

    # ── Node builders ────────────────────────────────────────────────────

    def sink(self, port=3080, path="/trigger", name=None):
        """Add an HttpSink node (webhook trigger endpoint).

        Args:
            port: TCP port to listen on.
            path: URL path for the trigger endpoint.
            name: Node name (default: http_sink).

        Returns:
            self for chaining.
        """
        node_name = name or "http_sink"
        self._nodes[node_name] = {
            "type": "http_sink",
            "port": port,
            "path": path,
        }
        self.port = port
        return self

    def source(self, url, format="sse", name=None, json_tap=None,
               post_body=None, dialect=None):
        """Add an HttpSource node (upstream inference endpoint).

        Args:
            url: Upstream URL to connect to.
            format: Response format (sse, json, raw).
            name: Node name (default: http_source).
            json_tap: JSONPath-like expression to extract data.
            post_body: Request body to POST upstream.
            dialect: SSE dialect name (openai, anthropic, ollama, etc.).

        Returns:
            self for chaining.
        """
        node_name = name or "http_source"
        node = {
            "type": "http_source",
            "url": url,
            "format": format,
        }
        if json_tap:
            node["json_tap"] = json_tap
        if post_body:
            node["post_body"] = post_body
        if dialect:
            node["dialect"] = dialect
        self._nodes[node_name] = node
        return self

    def transform(self, name, fn_body=None):
        """Add a transform node with inline code.

        Args:
            name: Node name.
            fn_body: Rust expression or handler body string.

        Returns:
            self for chaining.
        """
        node = {"type": "transform"}
        if fn_body:
            node["code"] = {"mode": "expr", "body": fn_body}
        self._nodes[name] = node
        return self

    def route(self, src_port, dst_port, mode="LoanWrite"):
        """Add a route between node ports.

        Args:
            src_port: Source port (e.g. "http_sink.data_out").
            dst_port: Destination port (e.g. "http_source.data_in").
            mode: Transfer mode (LoanWrite, Copy).

        Returns:
            self for chaining.
        """
        self._routes.append({"from": src_port, "to": dst_port, "mode": mode})
        return self

    # ── Semantic type declarations ───────────────────────────────────────

    def semantic_type(self, name, kind, fields=None, variants=None):
        """Declare a semantic type (state/event/fault/decision)."""
        self._semantic_types.append(
            _make_semantic_entry(name, kind, fields, variants)
        )
        return self

    def state(self, name, **fields):
        """Shorthand: declare a semantic state type AND set service state."""
        self.semantic_type(name, "state", fields=fields)
        self._state = {
            "type": "private_heap",
            "fields": [{"name": n, "type": t} for n, t in fields.items()],
        }
        return self

    def event(self, name, **fields):
        """Shorthand: declare a semantic event type."""
        return self.semantic_type(name, "event", fields=fields)

    def fault(self, name, variants=None):
        """Shorthand: declare a semantic fault type."""
        return self.semantic_type(name, "fault", variants=variants or [])

    def decision(self, name, **fields):
        """Shorthand: declare a semantic decision type."""
        return self.semantic_type(name, "decision", fields=fields)

    def failover(self, primary, backup, strategy="immediate"):
        """Declare a failover entry."""
        self._failover.append({
            "primary": primary, "backup": backup, "strategy": strategy,
        })
        return self

    def sse_event(self, name, fields, topic=None):
        """Declare an SSE event type."""
        self._sse_events.append({
            "name": name, "topic": topic,
            "fields": [{"name": n, "type": t} for n, t in fields.items()],
        })
        return self

    def ws_event(self, name, topic=None, **kwargs):
        """Declare a WebSocket event type."""
        self._ws_events.append({
            "name": name, "topic": topic,
            "fields": [{"name": n, "type": t} for n, t in kwargs.items()],
        })
        return self

    # ── YAML generation ──────────────────────────────────────────────────

    def to_yaml(self):
        """Generate YAML manifest string for ``vil compile``.

        Returns:
            YAML string matching WorkflowManifest format.
        """
        lines = []
        lines.append('vil_version: "6.0.0"')
        lines.append(f"name: {self.name}")
        lines.append(f"port: {self.port}")
        lines.append(f"token: {self.token}")

        lines.extend(_yaml_semantic_types(self._semantic_types))
        lines.extend(_yaml_errors(self._errors))
        lines.extend(_yaml_state(self._state))
        lines.extend(_yaml_failover(self._failover))
        lines.extend(_yaml_events(self._sse_events, "sse_events"))
        lines.extend(_yaml_events(self._ws_events, "ws_events"))

        # Nodes
        if self._nodes:
            lines.append("")
            lines.append("nodes:")
            for node_name, node in self._nodes.items():
                lines.append(f"  {node_name}:")
                lines.append(f"    type: {node['type']}")
                if node.get("port"):
                    lines.append(f"    port: {node['port']}")
                if node.get("path"):
                    lines.append(f'    path: "{node["path"]}"')
                if node.get("url"):
                    lines.append(f'    url: "{node["url"]}"')
                if node.get("format"):
                    lines.append(f"    format: {node['format']}")
                if node.get("json_tap"):
                    lines.append(f'    json_tap: "{node["json_tap"]}"')
                if node.get("dialect"):
                    lines.append(f"    dialect: {node['dialect']}")
                if node.get("post_body"):
                    lines.append(
                        f"    post_body: {json.dumps(node['post_body'])}"
                    )
                if node.get("code"):
                    code = node["code"]
                    lines.append("    code:")
                    lines.append(f"      mode: {code['mode']}")
                    lines.append(f'      body: "{code["body"]}"')

        # Routes
        if self._routes:
            lines.append("")
            lines.append("routes:")
            for r in self._routes:
                lines.append(f"  - from: {r['from']}")
                lines.append(f"    to: {r['to']}")
                lines.append(f"    mode: {r['mode']}")

        return "\n".join(lines) + "\n"

    def compile(self, release=True):
        """Call ``vil compile`` with the generated YAML manifest.

        Writes the YAML to stdout in manifest mode, or invokes the
        vil CLI compiler.

        Args:
            release: Build in release mode (default True).
        """
        if os.environ.get("VIL_COMPILE_MODE") == "manifest":
            sys.stdout.write(self.to_yaml())
            return

        # Write manifest to temp file and invoke vil compile
        import tempfile
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as f:
            f.write(self.to_yaml())
            manifest_path = f.name

        cmd = ["vil", "compile", "--manifest", manifest_path]
        if release:
            cmd.append("--release")
        cmd.extend(["--output", self.name])

        print(f"  Compiling pipeline: {self.name}")
        print(f"  Command: {' '.join(cmd)}")
        try:
            subprocess.run(cmd, check=True)
        except FileNotFoundError:
            print("\n  'vil' CLI not found. Install with: cargo install vil_cli")
            print(f"  Manifest written to: {manifest_path}")
        except subprocess.CalledProcessError as e:
            print(f"\n  Compilation failed (exit code {e.returncode})")
            print(f"  Manifest: {manifest_path}")


# =============================================================================
# ServiceProcess — VX service builder for VilServer
# =============================================================================


class ServiceProcess:
    """VX ServiceProcess builder for VilServer.

    Defines a named service with endpoints and semantic contract
    declarations (emits, faults, manages).

    Example::

        chat = ServiceProcess("chat")
        chat.endpoint("POST", "/query", "query_handler")
        chat.state("ChatState")
        chat.emits("ChatEvent")
        chat.faults("ChatFault")
    """

    def __init__(self, name):
        self.name = name
        self.prefix = f"/api/{name}"
        self._endpoints = []
        self._state_type = None
        self._emits_type = None
        self._faults_type = None
        self._semantic_types = []

    def endpoint(self, method, path, handler_name=None):
        """Add an endpoint to this service.

        Args:
            method: HTTP method (GET, POST, PUT, DELETE).
            path: URL path (appended to prefix).
            handler_name: Handler function name for codegen.

        Returns:
            self for chaining.
        """
        if handler_name is None:
            handler_name = path.strip("/").replace("/", "_").replace(":", "")
            if not handler_name:
                handler_name = "index"
            handler_name = f"{method.lower()}_{handler_name}"
        self._endpoints.append({
            "method": method, "path": path, "handler": handler_name,
        })
        return self

    def state(self, type_name):
        """Declare managed state type (semantic contract).

        Args:
            type_name: Name of the semantic state type.

        Returns:
            self for chaining.
        """
        self._state_type = type_name
        return self

    def emits(self, type_name):
        """Declare emitted event type (semantic contract).

        Args:
            type_name: Name of the semantic event type.

        Returns:
            self for chaining.
        """
        self._emits_type = type_name
        return self

    def faults(self, type_name):
        """Declare fault type (semantic contract).

        Args:
            type_name: Name of the semantic fault type.

        Returns:
            self for chaining.
        """
        self._faults_type = type_name
        return self


# =============================================================================
# VilServer — Server DSL -> YAML manifest -> native binary with VIL Way
# =============================================================================


class VilServer:
    """Server DSL -> YAML manifest -> native binary with VIL Way handlers.

    Generates a YAML manifest with endpoints: section.  ``vil compile``
    transpiles it to Rust code using VIL Way patterns:
    - ctx: ServiceCtx (not Extension<T>)
    - body: ShmSlice (not Json<T>)
    - .state(x) (not .extension(x))
    - body.json::<T>() for deserialization

    Example::

        app = VilServer("hello", port=8080)
        app.get("/greet/:name", output={"message": string()})
        app.post("/chat", input={"prompt": string(required=True)},
                 upstream=sse("http://localhost:4545/v1/chat"))
        app.compile()
    """

    def __init__(self, name, port=8080):
        self.name = name
        self.port = port
        self._services = []
        self._endpoints = []
        self._semantic_types = []
        self._errors = []
        self._state = None
        self._mesh = None
        self._failover = []
        self._sse_events = []
        self._ws_events = []

    # ── HTTP method registration ─────────────────────────────────────────

    def get(self, path, input=None, output=None, upstream=None,
            handler=None, exec_class=None):
        """Register a GET endpoint.

        Args:
            path: URL path.
            input: Input schema dict.
            output: Output schema dict.
            upstream: Upstream spec (from sse() or http()).
            handler: Handler function name (auto-generated if None).
            exec_class: Execution class override.

        Returns:
            self for chaining.
        """
        self._add_endpoint("GET", path, input, output, upstream,
                           handler, exec_class)
        return self

    def post(self, path, input=None, output=None, upstream=None,
             handler=None, exec_class=None):
        """Register a POST endpoint."""
        self._add_endpoint("POST", path, input, output, upstream,
                           handler, exec_class)
        return self

    def put(self, path, input=None, output=None, upstream=None,
            handler=None, exec_class=None):
        """Register a PUT endpoint."""
        self._add_endpoint("PUT", path, input, output, upstream,
                           handler, exec_class)
        return self

    def delete(self, path, handler=None, exec_class=None):
        """Register a DELETE endpoint."""
        self._add_endpoint("DELETE", path, None, None, None,
                           handler, exec_class)
        return self

    def _add_endpoint(self, method, path, input, output, upstream,
                      handler, exec_class):
        """Internal: add an endpoint to the manifest."""
        if handler is None:
            slug = path.strip("/").replace("/", "_").replace(":", "")
            if not slug:
                slug = "index"
            handler = f"{method.lower()}_{slug}"
        ep = {
            "method": method,
            "path": path,
            "handler": handler,
            "input": _build_schema(input) if input else None,
            "output": _build_schema(output) if output else None,
            "upstream": upstream,
        }
        if exec_class:
            ep["exec_class"] = exec_class
        self._endpoints.append(ep)

    # ── Semantic type declarations ───────────────────────────────────────

    def semantic_type(self, name, kind, fields=None, variants=None):
        """Declare a semantic type (state/event/fault/decision)."""
        self._semantic_types.append(
            _make_semantic_entry(name, kind, fields, variants)
        )
        return self

    def state(self, name, **fields):
        """Declare a semantic state type AND set service state.

        This declares the type in semantic_types AND sets the state
        section used by codegen to generate .state() calls.

        Args:
            name: State type name.
            **fields: Field name=type pairs.

        Returns:
            self for chaining.
        """
        self.semantic_type(name, "state", fields=fields)
        self._state = {
            "type": "private_heap",
            "fields": [{"name": n, "type": t} for n, t in fields.items()],
        }
        return self

    def event(self, name, **fields):
        """Shorthand: declare a semantic event type."""
        return self.semantic_type(name, "event", fields=fields)

    def fault(self, name, variants=None):
        """Shorthand: declare a semantic fault type."""
        return self.semantic_type(name, "fault", variants=variants or [])

    def decision(self, name, **fields):
        """Shorthand: declare a semantic decision type."""
        return self.semantic_type(name, "decision", fields=fields)

    def error(self, name, status, code=None, retry=None, fields=None):
        """Declare a VilError variant."""
        self._errors.append({
            "name": name, "status": status,
            "code": code, "retry": retry,
            "fields": [{"name": n, "type": t}
                        for n, t in (fields or {}).items()],
        })
        return self

    # ── Mesh / Failover ──────────────────────────────────────────────────

    def mesh(self, routes):
        """Declare Tri-Lane mesh routes.

        Args:
            routes: list of dicts {"from": str, "to": str, "lane": str}

        Returns:
            self for chaining.
        """
        self._mesh = {"routes": routes}
        return self

    def failover(self, primary, backup, strategy="immediate"):
        """Declare a failover entry."""
        self._failover.append({
            "primary": primary, "backup": backup, "strategy": strategy,
        })
        return self

    # ── Event declarations ───────────────────────────────────────────────

    def sse_event(self, name, fields, topic=None):
        """Declare an SSE event type."""
        self._sse_events.append({
            "name": name, "topic": topic,
            "fields": [{"name": n, "type": t} for n, t in fields.items()],
        })
        return self

    def ws_event(self, name, topic=None, **kwargs):
        """Declare a WebSocket event type."""
        self._ws_events.append({
            "name": name, "topic": topic,
            "fields": [{"name": n, "type": t} for n, t in kwargs.items()],
        })
        return self

    # ── ServiceProcess composition ───────────────────────────────────────

    def service_process(self, name, prefix=None):
        """Create and register a VX ServiceProcess.

        Args:
            name: Service name.
            prefix: URL prefix (default: /api/<name>).

        Returns:
            The new ServiceProcess for further configuration.
        """
        svc = ServiceProcess(name)
        if prefix:
            svc.prefix = prefix
        self._services.append(svc)
        return svc

    # ── YAML generation ──────────────────────────────────────────────────

    def to_yaml(self):
        """Generate YAML manifest string for ``vil compile``.

        Returns:
            YAML string matching WorkflowManifest format.
        """
        lines = []
        lines.append('vil_version: "6.0.0"')
        lines.append(f"name: {self.name}")
        lines.append(f"port: {self.port}")

        lines.extend(_yaml_semantic_types(self._semantic_types))
        lines.extend(_yaml_errors(self._errors))
        lines.extend(_yaml_state(self._state))

        # Mesh
        if self._mesh:
            lines.append("mesh:")
            lines.append("  routes:")
            for r in self._mesh["routes"]:
                lines.append(f"    - from: {r['from']}")
                lines.append(f"      to: {r['to']}")
                lines.append(f"      lane: {r['lane']}")

        lines.extend(_yaml_failover(self._failover))
        lines.extend(_yaml_events(self._sse_events, "sse_events"))
        lines.extend(_yaml_events(self._ws_events, "ws_events"))

        # Endpoints (server mode)
        if self._endpoints:
            lines.append("endpoints:")
            for ep in self._endpoints:
                lines.append(f"  - method: {ep['method']}")
                lines.append(f'    path: "{ep["path"]}"')
                lines.append(f"    handler: {ep['handler']}")
                if ep.get("exec_class"):
                    lines.append(f"    exec_class: {ep['exec_class']}")
                if ep.get("input"):
                    inp = ep["input"]
                    lines.append("    input:")
                    lines.append(f"      type: {inp['type']}")
                    lines.append("      fields:")
                    lines.extend(_yaml_fields(inp["fields"], indent=8))
                if ep.get("output"):
                    out = ep["output"]
                    lines.append("    output:")
                    lines.append(f"      type: {out['type']}")
                    lines.append("      fields:")
                    lines.extend(_yaml_fields(out["fields"], indent=8))
                if ep.get("upstream"):
                    u = ep["upstream"]
                    lines.append("    upstream:")
                    lines.append(f"      type: {u['type']}")
                    lines.append(f'      url: "{u["url"]}"')
                    if u.get("method"):
                        lines.append(f"      method: {u['method']}")
                    if u.get("body_template"):
                        lines.append(
                            f"      body_template: "
                            f"{json.dumps(u['body_template'])}"
                        )

        # Services (VX app mode)
        if self._services:
            lines.append("services:")
            for svc in self._services:
                lines.append(f"  - name: {svc.name}")
                lines.append(f'    prefix: "{svc.prefix}"')
                if svc._emits_type:
                    lines.append(f"    emits: {svc._emits_type}")
                if svc._faults_type:
                    lines.append(f"    faults: {svc._faults_type}")
                if svc._state_type:
                    lines.append(f"    manages: {svc._state_type}")
                if svc._endpoints:
                    lines.append("    endpoints:")
                    for ep in svc._endpoints:
                        lines.append(f"      - method: {ep['method']}")
                        lines.append(f'        path: "{ep["path"]}"')
                        lines.append(f"        handler: {ep['handler']}")

        return "\n".join(lines) + "\n"

    def compile(self, release=True):
        """Call ``vil compile`` with the generated YAML manifest.

        Args:
            release: Build in release mode (default True).
        """
        if os.environ.get("VIL_COMPILE_MODE") == "manifest":
            sys.stdout.write(self.to_yaml())
            return

        import tempfile
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as f:
            f.write(self.to_yaml())
            manifest_path = f.name

        cmd = ["vil", "compile", "--manifest", manifest_path]
        if release:
            cmd.append("--release")
        cmd.extend(["--output", self.name])

        print(f"  Compiling server: {self.name}")
        print(f"  Command: {' '.join(cmd)}")
        try:
            subprocess.run(cmd, check=True)
        except FileNotFoundError:
            print("\n  'vil' CLI not found. Install with: cargo install vil_cli")
            print(f"  Manifest written to: {manifest_path}")
        except subprocess.CalledProcessError as e:
            print(f"\n  Compilation failed (exit code {e.returncode})")
            print(f"  Manifest: {manifest_path}")


# =============================================================================
# VilApp — High-level VX application builder
# =============================================================================


class VilApp:
    """VX process-oriented application builder.

    Composes ServiceProcess instances into a single application manifest.

    Example::

        app = VilApp("my-ai-gateway")
        chat = app.service("chat")
        chat.endpoint("POST", "/query", "query_handler")
        chat.emits("ChatEvent")
        app.compile()
    """

    def __init__(self, name, port=8080):
        self.name = name
        self.port = port
        self._services = []
        self._semantic_types = []

    def service(self, name_or_svc):
        """Add a ServiceProcess.

        Args:
            name_or_svc: ServiceProcess instance or service name string.

        Returns:
            The ServiceProcess (created or passed in).
        """
        if isinstance(name_or_svc, str):
            svc = ServiceProcess(name_or_svc)
            self._services.append(svc)
            return svc
        self._services.append(name_or_svc)
        return name_or_svc

    def semantic_type(self, name, kind, fields=None, variants=None):
        """Declare a semantic type at the app level."""
        self._semantic_types.append(
            _make_semantic_entry(name, kind, fields, variants)
        )
        return self

    def state(self, name, **fields):
        """Shorthand: declare a semantic state type."""
        return self.semantic_type(name, "state", fields=fields)

    def event(self, name, **fields):
        """Shorthand: declare a semantic event type."""
        return self.semantic_type(name, "event", fields=fields)

    def fault(self, name, variants=None):
        """Shorthand: declare a semantic fault type."""
        return self.semantic_type(name, "fault", variants=variants or [])

    def to_yaml(self):
        """Generate YAML manifest string."""
        lines = []
        lines.append('vil_version: "6.0.0"')
        lines.append(f"name: {self.name}")
        lines.append(f"port: {self.port}")
        lines.append("mode: vil_app")

        lines.extend(_yaml_semantic_types(self._semantic_types))

        lines.append("")
        lines.append("services:")
        for svc in self._services:
            lines.append(f"  - name: {svc.name}")
            lines.append(f'    prefix: "{svc.prefix}"')
            if svc._emits_type:
                lines.append(f"    emits: {svc._emits_type}")
            if svc._faults_type:
                lines.append(f"    faults: {svc._faults_type}")
            if svc._state_type:
                lines.append(f"    manages: {svc._state_type}")
            if svc._endpoints:
                lines.append("    endpoints:")
                for ep in svc._endpoints:
                    lines.append(f"      - method: {ep['method']}")
                    lines.append(f'        path: "{ep["path"]}"')
                    lines.append(f"        handler: {ep['handler']}")

        return "\n".join(lines) + "\n"

    def compile(self, release=True):
        """Call ``vil compile`` with the generated YAML manifest."""
        if os.environ.get("VIL_COMPILE_MODE") == "manifest":
            sys.stdout.write(self.to_yaml())
            return

        import tempfile
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".yaml", delete=False
        ) as f:
            f.write(self.to_yaml())
            manifest_path = f.name

        cmd = ["vil", "compile", "--manifest", manifest_path]
        if release:
            cmd.append("--release")
        cmd.extend(["--output", self.name])

        print(f"  Compiling app: {self.name}")
        print(f"  Command: {' '.join(cmd)}")
        try:
            subprocess.run(cmd, check=True)
        except FileNotFoundError:
            print("\n  'vil' CLI not found. Install with: cargo install vil_cli")
            print(f"  Manifest written to: {manifest_path}")
        except subprocess.CalledProcessError as e:
            print(f"\n  Compilation failed (exit code {e.returncode})")
            print(f"  Manifest: {manifest_path}")
