// Package vil provides a declarative SDK for building VIL pipelines and servers.
//
// It generates YAML manifests consumed by `vil compile --from go`.
// When the environment variable VIL_COMPILE_MODE is set to "manifest",
// the SDK prints the YAML to stdout and exits. Otherwise it invokes `vil compile`.
//
// No external dependencies -- pure Go stdlib.
package vil

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
)

// ---------------------------------------------------------------------------
// Field type helpers
// ---------------------------------------------------------------------------

// FieldSpec describes a schema field declaration.
type FieldSpec struct {
	Type     string
	Required bool
}

// String declares a String field.
func String(required bool) FieldSpec {
	return FieldSpec{Type: "String", Required: required}
}

// Number declares a u64 field.
func Number(required bool) FieldSpec {
	return FieldSpec{Type: "u64", Required: required}
}

// Boolean declares a bool field.
func Boolean(required bool) FieldSpec {
	return FieldSpec{Type: "bool", Required: required}
}

// Array declares a Vec<T> field.
func Array(items string) FieldSpec {
	return FieldSpec{Type: fmt.Sprintf("Vec<%s>", items), Required: false}
}

// ---------------------------------------------------------------------------
// Upstream helpers
// ---------------------------------------------------------------------------

// UpstreamSpec describes an upstream connection (SSE or HTTP).
type UpstreamSpec struct {
	Type         string
	URL          string
	Method       string
	BodyTemplate string
}

// SSE declares an SSE upstream connection.
func SSE(url string, body ...string) UpstreamSpec {
	u := UpstreamSpec{Type: "sse", URL: url}
	if len(body) > 0 {
		u.BodyTemplate = body[0]
	}
	return u
}

// HTTP declares an HTTP upstream connection.
func HTTP(url, method string, body ...string) UpstreamSpec {
	u := UpstreamSpec{Type: "http", URL: url, Method: method}
	if len(body) > 0 {
		u.BodyTemplate = body[0]
	}
	return u
}

// ---------------------------------------------------------------------------
// Internal YAML helpers
// ---------------------------------------------------------------------------

type field struct {
	Name     string
	Type     string
	Required bool
}

type semanticEntry struct {
	Name     string
	Kind     string
	Fields   []field
	Variants []string
}

type errorEntry struct {
	Name   string
	Status int
	Code   string
	Retry  *bool
	Fields []field
}

type failoverEntry struct {
	Primary  string
	Backup   string
	Strategy string
}

type eventEntry struct {
	Name   string
	Topic  string
	Fields []field
}

type stateSection struct {
	Type   string
	Fields []field
}

func fieldsFromMap(m map[string]string) []field {
	// Preserve insertion order is not guaranteed in Go maps.
	// Callers who care about order should use a slice-based API.
	fields := make([]field, 0, len(m))
	for n, t := range m {
		fields = append(fields, field{Name: n, Type: t})
	}
	return fields
}

func yamlFields(fields []field, indent int) []string {
	prefix := strings.Repeat(" ", indent)
	var lines []string
	for _, f := range fields {
		lines = append(lines, fmt.Sprintf("%s- name: %s", prefix, f.Name))
		lines = append(lines, fmt.Sprintf("%s  type: %s", prefix, f.Type))
		if f.Required {
			lines = append(lines, fmt.Sprintf("%s  required: true", prefix))
		}
	}
	return lines
}

func yamlSemanticTypes(types []semanticEntry) []string {
	if len(types) == 0 {
		return nil
	}
	lines := []string{"semantic_types:"}
	for _, st := range types {
		lines = append(lines, fmt.Sprintf("  - name: %s", st.Name))
		lines = append(lines, fmt.Sprintf("    kind: %s", st.Kind))
		if len(st.Fields) > 0 {
			lines = append(lines, "    fields:")
			lines = append(lines, yamlFields(st.Fields, 6)...)
		}
		if len(st.Variants) > 0 {
			lines = append(lines, "    variants:")
			for _, v := range st.Variants {
				lines = append(lines, fmt.Sprintf("      - %s", v))
			}
		}
	}
	return lines
}

func yamlErrors(errors []errorEntry) []string {
	if len(errors) == 0 {
		return nil
	}
	lines := []string{"errors:"}
	for _, err := range errors {
		lines = append(lines, fmt.Sprintf("  - name: %s", err.Name))
		lines = append(lines, fmt.Sprintf("    status: %d", err.Status))
		if err.Code != "" {
			lines = append(lines, fmt.Sprintf("    code: %s", err.Code))
		}
		if err.Retry != nil {
			if *err.Retry {
				lines = append(lines, "    retry: true")
			} else {
				lines = append(lines, "    retry: false")
			}
		}
		if len(err.Fields) > 0 {
			lines = append(lines, "    fields:")
			lines = append(lines, yamlFields(err.Fields, 6)...)
		}
	}
	return lines
}

func yamlState(s *stateSection) []string {
	if s == nil {
		return nil
	}
	lines := []string{"state:"}
	lines = append(lines, fmt.Sprintf("  type: %s", s.Type))
	lines = append(lines, "  fields:")
	lines = append(lines, yamlFields(s.Fields, 4)...)
	return lines
}

func yamlFailover(entries []failoverEntry) []string {
	if len(entries) == 0 {
		return nil
	}
	lines := []string{"failover:", "  entries:"}
	for _, e := range entries {
		lines = append(lines, fmt.Sprintf("    - primary: %s", e.Primary))
		lines = append(lines, fmt.Sprintf("      backup: %s", e.Backup))
		lines = append(lines, fmt.Sprintf("      strategy: %s", e.Strategy))
	}
	return lines
}

func yamlEvents(events []eventEntry, sectionName string) []string {
	if len(events) == 0 {
		return nil
	}
	lines := []string{fmt.Sprintf("%s:", sectionName)}
	for _, ev := range events {
		lines = append(lines, fmt.Sprintf("  - name: %s", ev.Name))
		if ev.Topic != "" {
			lines = append(lines, fmt.Sprintf("    topic: %s", ev.Topic))
		}
		lines = append(lines, "    fields:")
		lines = append(lines, yamlFields(ev.Fields, 6)...)
	}
	return lines
}

// jsonEscape produces a JSON-escaped string representation (with quotes).
func jsonEscape(s string) string {
	s = strings.ReplaceAll(s, `\`, `\\`)
	s = strings.ReplaceAll(s, `"`, `\"`)
	s = strings.ReplaceAll(s, "\n", `\n`)
	s = strings.ReplaceAll(s, "\t", `\t`)
	return `"` + s + `"`
}

// compile writes the YAML to a temp file and invokes `vil compile`.
func compile(name, yaml string, release bool) {
	if os.Getenv("VIL_COMPILE_MODE") == "manifest" {
		fmt.Print(yaml)
		return
	}

	tmpFile, err := os.CreateTemp("", "vil-*.yaml")
	if err != nil {
		fmt.Fprintf(os.Stderr, "  Failed to create temp file: %v\n", err)
		return
	}
	manifestPath := tmpFile.Name()
	_, _ = tmpFile.WriteString(yaml)
	_ = tmpFile.Close()

	args := []string{"compile", "--manifest", manifestPath}
	if release {
		args = append(args, "--release")
	}
	args = append(args, "--output", name)

	fmt.Printf("  Compiling: %s\n", name)
	fmt.Printf("  Command: vil %s\n", strings.Join(args, " "))

	cmd := exec.Command("vil", args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		if _, ok := err.(*exec.ExitError); ok {
			fmt.Fprintf(os.Stderr, "\n  Compilation failed\n")
			fmt.Fprintf(os.Stderr, "  Manifest: %s\n", manifestPath)
		} else {
			fmt.Fprintf(os.Stderr, "\n  'vil' CLI not found. Install with: cargo install vil_cli\n")
			fmt.Fprintf(os.Stderr, "  Manifest written to: %s\n", manifestPath)
		}
	}
}

// ---------------------------------------------------------------------------
// SinkOpts / SourceOpts
// ---------------------------------------------------------------------------

// SinkOpts configures an HttpSink node.
type SinkOpts struct {
	Port int
	Path string
	Name string
}

// SourceOpts configures an HttpSource node.
type SourceOpts struct {
	URL      string
	Format   string
	Name     string
	JSONTap  string // preferred
	JsonTap  string // alias (backward compat)
	PostBody string
	Dialect  string
}

// ---------------------------------------------------------------------------
// VilPipeline
// ---------------------------------------------------------------------------

type pipelineNode struct {
	Type     string
	Port     int
	Path     string
	URL      string
	Format   string
	JSONTap  string
	PostBody string
	Dialect  string
	Code     *codeBlock
}

type codeBlock struct {
	Mode string
	Body string
}

type routeEntry struct {
	From string
	To   string
	Mode string
}

// VilPipeline is the declarative pipeline DSL that produces YAML manifests.
type VilPipeline struct {
	name          string
	port          int
	token         string
	nodeOrder     []string
	nodes         map[string]*pipelineNode
	routes        []routeEntry
	semanticTypes []semanticEntry
	errors        []errorEntry
	state         *stateSection
	failover      []failoverEntry
	sseEvents     []eventEntry
	wsEvents      []eventEntry
}

// NewPipeline creates a new VilPipeline with the given name and port.
func NewPipeline(name string, port int) *VilPipeline {
	return &VilPipeline{
		name:  name,
		port:  port,
		token: "shm",
		nodes: make(map[string]*pipelineNode),
	}
}

// Sink adds an HttpSink node (webhook trigger endpoint).
func (p *VilPipeline) Sink(opts SinkOpts) *VilPipeline {
	nodeName := opts.Name
	if nodeName == "" {
		nodeName = "http_sink"
	}
	port := opts.Port
	if port == 0 {
		port = 3080
	}
	path := opts.Path
	if path == "" {
		path = "/trigger"
	}
	p.nodes[nodeName] = &pipelineNode{
		Type: "http_sink",
		Port: port,
		Path: path,
	}
	if _, exists := nodeOrderContains(p.nodeOrder, nodeName); !exists {
		p.nodeOrder = append(p.nodeOrder, nodeName)
	}
	p.port = port
	return p
}

// Source adds an HttpSource node (upstream inference endpoint).
func (p *VilPipeline) Source(opts SourceOpts) *VilPipeline {
	nodeName := opts.Name
	if nodeName == "" {
		nodeName = "http_source"
	}
	format := opts.Format
	if format == "" {
		format = "sse"
	}
	node := &pipelineNode{
		Type:   "http_source",
		URL:    opts.URL,
		Format: format,
	}
	jsonTap := opts.JSONTap
	if jsonTap == "" {
		jsonTap = opts.JsonTap // alias
	}
	if jsonTap != "" {
		node.JSONTap = jsonTap
	}
	if opts.PostBody != "" {
		node.PostBody = opts.PostBody
	}
	if opts.Dialect != "" {
		node.Dialect = opts.Dialect
	}
	p.nodes[nodeName] = node
	if _, exists := nodeOrderContains(p.nodeOrder, nodeName); !exists {
		p.nodeOrder = append(p.nodeOrder, nodeName)
	}
	return p
}

// Transform adds a transform node with inline code.
func (p *VilPipeline) Transform(name string, fnBody string) *VilPipeline {
	node := &pipelineNode{Type: "transform"}
	if fnBody != "" {
		node.Code = &codeBlock{Mode: "expr", Body: fnBody}
	}
	p.nodes[name] = node
	if _, exists := nodeOrderContains(p.nodeOrder, name); !exists {
		p.nodeOrder = append(p.nodeOrder, name)
	}
	return p
}

// Route adds a route between node ports.
func (p *VilPipeline) Route(src, dst, mode string) *VilPipeline {
	p.routes = append(p.routes, routeEntry{From: src, To: dst, Mode: mode})
	return p
}

// SemanticType declares a semantic type (state/event/fault/decision).
// Variants is optional — pass nil or omit for non-fault types.
func (p *VilPipeline) SemanticType(name, kind string, fields map[string]string, variants ...[]string) *VilPipeline {
	var v []string
	if len(variants) > 0 {
		v = variants[0]
	}
	p.semanticTypes = append(p.semanticTypes, semanticEntry{
		Name:     name,
		Kind:     kind,
		Fields:   fieldsFromMap(fields),
		Variants: v,
	})
	return p
}

// State declares a semantic state type and sets service state.
func (p *VilPipeline) State(name string, fields map[string]string) *VilPipeline {
	p.SemanticType(name, "state", fields, nil)
	f := fieldsFromMap(fields)
	p.state = &stateSection{Type: "private_heap", Fields: f}
	return p
}

// Event declares a semantic event type.
func (p *VilPipeline) Event(name string, fields map[string]string) *VilPipeline {
	return p.SemanticType(name, "event", fields, nil)
}

// Fault declares a semantic fault type.
func (p *VilPipeline) Fault(name string, variants []string) *VilPipeline {
	return p.SemanticType(name, "fault", nil, variants)
}

// Failover declares a failover entry.
func (p *VilPipeline) Failover(primary, backup, strategy string) *VilPipeline {
	p.failover = append(p.failover, failoverEntry{
		Primary: primary, Backup: backup, Strategy: strategy,
	})
	return p
}

// SseEvent declares an SSE event type.
func (p *VilPipeline) SseEvent(name string, fields map[string]string, topic string) *VilPipeline {
	p.sseEvents = append(p.sseEvents, eventEntry{
		Name:   name,
		Topic:  topic,
		Fields: fieldsFromMap(fields),
	})
	return p
}

// ToYaml generates the YAML manifest string.
func (p *VilPipeline) ToYaml() string {
	var lines []string
	lines = append(lines, `vil_version: "6.0.0"`)
	lines = append(lines, fmt.Sprintf("name: %s", p.name))
	lines = append(lines, fmt.Sprintf("port: %d", p.port))
	lines = append(lines, fmt.Sprintf("token: %s", p.token))

	lines = append(lines, yamlSemanticTypes(p.semanticTypes)...)
	lines = append(lines, yamlErrors(p.errors)...)
	lines = append(lines, yamlState(p.state)...)
	lines = append(lines, yamlFailover(p.failover)...)
	lines = append(lines, yamlEvents(p.sseEvents, "sse_events")...)
	lines = append(lines, yamlEvents(p.wsEvents, "ws_events")...)

	// Nodes
	if len(p.nodeOrder) > 0 {
		lines = append(lines, "")
		lines = append(lines, "nodes:")
		for _, nodeName := range p.nodeOrder {
			node := p.nodes[nodeName]
			lines = append(lines, fmt.Sprintf("  %s:", nodeName))
			lines = append(lines, fmt.Sprintf("    type: %s", node.Type))
			if node.Port != 0 {
				lines = append(lines, fmt.Sprintf("    port: %d", node.Port))
			}
			if node.Path != "" {
				lines = append(lines, fmt.Sprintf(`    path: "%s"`, node.Path))
			}
			if node.URL != "" {
				lines = append(lines, fmt.Sprintf(`    url: "%s"`, node.URL))
			}
			if node.Format != "" {
				lines = append(lines, fmt.Sprintf("    format: %s", node.Format))
			}
			if node.JSONTap != "" {
				lines = append(lines, fmt.Sprintf(`    json_tap: "%s"`, node.JSONTap))
			}
			if node.Dialect != "" {
				lines = append(lines, fmt.Sprintf("    dialect: %s", node.Dialect))
			}
			if node.PostBody != "" {
				lines = append(lines, fmt.Sprintf("    post_body: %s", jsonEscape(node.PostBody)))
			}
			if node.Code != nil {
				lines = append(lines, "    code:")
				lines = append(lines, fmt.Sprintf("      mode: %s", node.Code.Mode))
				lines = append(lines, fmt.Sprintf(`      body: "%s"`, node.Code.Body))
			}
		}
	}

	// Routes
	if len(p.routes) > 0 {
		lines = append(lines, "")
		lines = append(lines, "routes:")
		for _, r := range p.routes {
			lines = append(lines, fmt.Sprintf("  - from: %s", r.From))
			lines = append(lines, fmt.Sprintf("    to: %s", r.To))
			lines = append(lines, fmt.Sprintf("    mode: %s", r.Mode))
		}
	}

	return strings.Join(lines, "\n") + "\n"
}

// ToYAML is an alias for ToYaml (backward compat).
func (p *VilPipeline) ToYAML() string { return p.ToYaml() }

// Compile generates the YAML manifest and either prints it (manifest mode)
// or invokes `vil compile`.
func (p *VilPipeline) Compile() {
	compile(p.name, p.ToYaml(), true)
}

// ---------------------------------------------------------------------------
// ServiceProcess
// ---------------------------------------------------------------------------

type serviceEndpoint struct {
	Method  string
	Path    string
	Handler string
}

// ServiceProcess is a VX service builder for VilServer.
type ServiceProcess struct {
	name       string
	prefix     string
	endpoints  []serviceEndpoint
	stateType  string
	emitsType  string
	faultsType string
}

// NewService creates a new ServiceProcess with the given name.
func NewService(name string) *ServiceProcess {
	return &ServiceProcess{
		name:   name,
		prefix: fmt.Sprintf("/api/%s", name),
	}
}

// Endpoint adds an endpoint to this service.
func (sp *ServiceProcess) Endpoint(method, path string, handler ...string) *ServiceProcess {
	h := ""
	if len(handler) > 0 {
		h = handler[0]
	}
	if h == "" {
		slug := strings.TrimLeft(path, "/")
		slug = strings.ReplaceAll(slug, "/", "_")
		slug = strings.ReplaceAll(slug, ":", "")
		if slug == "" {
			slug = "index"
		}
		h = fmt.Sprintf("%s_%s", strings.ToLower(method), slug)
	}
	sp.endpoints = append(sp.endpoints, serviceEndpoint{
		Method: method, Path: path, Handler: h,
	})
	return sp
}

// SetState declares the managed state type (semantic contract).
func (sp *ServiceProcess) SetState(typeName string) *ServiceProcess {
	sp.stateType = typeName
	return sp
}

// Emits declares the emitted event type (semantic contract).
func (sp *ServiceProcess) Emits(typeName string) *ServiceProcess {
	sp.emitsType = typeName
	return sp
}

// Faults declares the fault type (semantic contract).
func (sp *ServiceProcess) Faults(typeName string) *ServiceProcess {
	sp.faultsType = typeName
	return sp
}

// ---------------------------------------------------------------------------
// VilServer
// ---------------------------------------------------------------------------

type serverEndpoint struct {
	Method    string
	Path      string
	Handler   string
	ExecClass string
	Input     *schemaSpec
	Output    *schemaSpec
	Upstream  *UpstreamSpec
}

type schemaSpec struct {
	Type   string
	Fields []field
}

func buildSchema(m map[string]FieldSpec) *schemaSpec {
	if len(m) == 0 {
		return nil
	}
	fields := make([]field, 0, len(m))
	for n, spec := range m {
		fields = append(fields, field{Name: n, Type: spec.Type, Required: spec.Required})
	}
	return &schemaSpec{Type: "json", Fields: fields}
}

type meshRoute struct {
	From string
	To   string
	Lane string
}

// VilServer is the server DSL that produces YAML manifests.
type VilServer struct {
	name          string
	port          int
	services      []*ServiceProcess
	endpoints     []serverEndpoint
	semanticTypes []semanticEntry
	errors        []errorEntry
	state         *stateSection
	mesh          *meshSection
	failover      []failoverEntry
	sseEvents     []eventEntry
	wsEvents      []eventEntry
	observerOn    bool
}

type meshSection struct {
	Routes []meshRoute
}

// NewServer creates a new VilServer with the given name and port.
func NewServer(name string, port int) *VilServer {
	return &VilServer{
		name: name,
		port: port,
	}
}

// Service registers a ServiceProcess with this server.
func (s *VilServer) Service(svc *ServiceProcess) *VilServer {
	s.services = append(s.services, svc)
	return s
}

// Observer enables or disables the observer.
func (s *VilServer) Observer(enabled bool) *VilServer {
	s.observerOn = enabled
	return s
}

// SemanticType declares a semantic type (state/event/fault/decision).
func (s *VilServer) SemanticType(name, kind string, fields map[string]string, variants []string) *VilServer {
	s.semanticTypes = append(s.semanticTypes, semanticEntry{
		Name:     name,
		Kind:     kind,
		Fields:   fieldsFromMap(fields),
		Variants: variants,
	})
	return s
}

// State declares a semantic state type and sets service state.
func (s *VilServer) State(name string, fields map[string]string) *VilServer {
	s.SemanticType(name, "state", fields, nil)
	f := fieldsFromMap(fields)
	s.state = &stateSection{Type: "private_heap", Fields: f}
	return s
}

// Error declares a VilError variant.
func (s *VilServer) Error(name string, status int, code string, retry *bool, fields map[string]string) *VilServer {
	s.errors = append(s.errors, errorEntry{
		Name:   name,
		Status: status,
		Code:   code,
		Retry:  retry,
		Fields: fieldsFromMap(fields),
	})
	return s
}

// ToYaml generates the YAML manifest string.
func (s *VilServer) ToYaml() string {
	var lines []string
	lines = append(lines, `vil_version: "6.0.0"`)
	lines = append(lines, fmt.Sprintf("name: %s", s.name))
	lines = append(lines, fmt.Sprintf("port: %d", s.port))

	lines = append(lines, yamlSemanticTypes(s.semanticTypes)...)
	lines = append(lines, yamlErrors(s.errors)...)
	lines = append(lines, yamlState(s.state)...)

	// Mesh
	if s.mesh != nil {
		lines = append(lines, "mesh:")
		lines = append(lines, "  routes:")
		for _, r := range s.mesh.Routes {
			lines = append(lines, fmt.Sprintf("    - from: %s", r.From))
			lines = append(lines, fmt.Sprintf("      to: %s", r.To))
			lines = append(lines, fmt.Sprintf("      lane: %s", r.Lane))
		}
	}

	lines = append(lines, yamlFailover(s.failover)...)
	lines = append(lines, yamlEvents(s.sseEvents, "sse_events")...)
	lines = append(lines, yamlEvents(s.wsEvents, "ws_events")...)

	// Endpoints (server mode)
	if len(s.endpoints) > 0 {
		lines = append(lines, "endpoints:")
		for _, ep := range s.endpoints {
			lines = append(lines, fmt.Sprintf("  - method: %s", ep.Method))
			lines = append(lines, fmt.Sprintf(`    path: "%s"`, ep.Path))
			lines = append(lines, fmt.Sprintf("    handler: %s", ep.Handler))
			if ep.ExecClass != "" {
				lines = append(lines, fmt.Sprintf("    exec_class: %s", ep.ExecClass))
			}
			if ep.Input != nil {
				lines = append(lines, "    input:")
				lines = append(lines, fmt.Sprintf("      type: %s", ep.Input.Type))
				lines = append(lines, "      fields:")
				lines = append(lines, yamlFields(ep.Input.Fields, 8)...)
			}
			if ep.Output != nil {
				lines = append(lines, "    output:")
				lines = append(lines, fmt.Sprintf("      type: %s", ep.Output.Type))
				lines = append(lines, "      fields:")
				lines = append(lines, yamlFields(ep.Output.Fields, 8)...)
			}
			if ep.Upstream != nil {
				u := ep.Upstream
				lines = append(lines, "    upstream:")
				lines = append(lines, fmt.Sprintf("      type: %s", u.Type))
				lines = append(lines, fmt.Sprintf(`      url: "%s"`, u.URL))
				if u.Method != "" {
					lines = append(lines, fmt.Sprintf("      method: %s", u.Method))
				}
				if u.BodyTemplate != "" {
					lines = append(lines, fmt.Sprintf("      body_template: %s", jsonEscape(u.BodyTemplate)))
				}
			}
		}
	}

	// Services (VX app mode)
	if len(s.services) > 0 {
		lines = append(lines, "services:")
		for _, svc := range s.services {
			lines = append(lines, fmt.Sprintf("  - name: %s", svc.name))
			lines = append(lines, fmt.Sprintf(`    prefix: "%s"`, svc.prefix))
			if svc.emitsType != "" {
				lines = append(lines, fmt.Sprintf("    emits: %s", svc.emitsType))
			}
			if svc.faultsType != "" {
				lines = append(lines, fmt.Sprintf("    faults: %s", svc.faultsType))
			}
			if svc.stateType != "" {
				lines = append(lines, fmt.Sprintf("    manages: %s", svc.stateType))
			}
			if len(svc.endpoints) > 0 {
				lines = append(lines, "    endpoints:")
				for _, ep := range svc.endpoints {
					lines = append(lines, fmt.Sprintf("      - method: %s", ep.Method))
					lines = append(lines, fmt.Sprintf(`        path: "%s"`, ep.Path))
					lines = append(lines, fmt.Sprintf("        handler: %s", ep.Handler))
				}
			}
		}
	}

	return strings.Join(lines, "\n") + "\n"
}

// Compile generates the YAML manifest and either prints it (manifest mode)
// or invokes `vil compile`.
func (s *VilServer) Compile() {
	compile(s.name, s.ToYaml(), true)
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

func nodeOrderContains(order []string, name string) (int, bool) {
	for i, n := range order {
		if n == name {
			return i, true
		}
	}
	return -1, false
}
