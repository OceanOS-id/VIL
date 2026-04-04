/**
 * VilServer + ServiceProcess — Server DSL -> YAML manifest -> native binary.
 *
 * Generates a YAML manifest with endpoints: section.
 * `vil compile` transpiles it to Rust code using VIL Way patterns:
 * - ctx: ServiceCtx (not Extension<T>)
 * - body: ShmSlice (not Json<T>)
 * - .state(x) (not .extension(x))
 * - body.json::<T>() for deserialization
 */

import { execFileSync } from 'child_process';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import {
  SemanticEntry,
  ErrorEntry,
  StateEntry,
  FailoverEntry,
  EventEntry,
  SchemaEntry,
  yamlSemanticTypes,
  yamlErrors,
  yamlState,
  yamlFailover,
  yamlEvents,
  yamlFields,
  buildSchema,
  makeSemanticEntry,
} from './yaml';
import { UpstreamSpec } from './types';

// ============================================================================
// HandlerImpl — handler implementation descriptor
// ============================================================================

/** Describes how a handler endpoint is implemented. */
export interface HandlerImpl {
  mode: 'sidecar' | 'wasm' | 'stub' | 'inline';
  command?: string;     // sidecar
  protocol?: string;    // sidecar (shm|http)
  timeout_ms?: number;  // sidecar
  module?: string;      // wasm
  function?: string;    // wasm
  response?: string;    // stub
  code?: string;        // inline
}

/** Create a sidecar handler implementation. */
export function sidecar(command: string, protocol = 'shm', timeout_ms = 5000): HandlerImpl {
  return { mode: 'sidecar', command, protocol, timeout_ms };
}

/** Create a WASM handler implementation. */
export function wasm(module: string, fn = 'handle'): HandlerImpl {
  return { mode: 'wasm', module, function: fn };
}

/** Create a stub handler implementation. */
export function stub(response = '{"ok": true}'): HandlerImpl {
  return { mode: 'stub', response };
}

/** Create an inline handler implementation. */
export function inline(code: string): HandlerImpl {
  return { mode: 'inline', code };
}

function yamlHandlerImpl(impl: HandlerImpl | undefined, indent: number): string[] {
  if (!impl) return [];
  const prefix = ' '.repeat(indent);
  const lines: string[] = [`${prefix}impl:`];
  lines.push(`${prefix}  mode: ${impl.mode}`);
  switch (impl.mode) {
    case 'inline':
      if (impl.code) {
        lines.push(`${prefix}  code: |`);
        for (const cl of impl.code.split('\n')) {
          lines.push(`${prefix}    ${cl}`);
        }
      }
      break;
    case 'wasm':
      if (impl.module) lines.push(`${prefix}  module: ${impl.module}`);
      if (impl.function) lines.push(`${prefix}  function: ${impl.function}`);
      break;
    case 'sidecar':
      if (impl.command) lines.push(`${prefix}  command: ${impl.command}`);
      if (impl.protocol) lines.push(`${prefix}  protocol: ${impl.protocol}`);
      if (impl.timeout_ms != null) lines.push(`${prefix}  timeout_ms: ${impl.timeout_ms}`);
      break;
    case 'stub':
      if (impl.response) lines.push(`${prefix}  response: '${impl.response}'`);
      break;
  }
  return lines;
}

// ============================================================================
// ServiceProcess
// ============================================================================

interface EndpointEntry {
  method: string;
  path: string;
  handler: string;
  impl?: HandlerImpl;
}

export class ServiceProcess {
  name: string;
  prefix: string;
  private _endpoints: EndpointEntry[] = [];
  private _stateType: string | null = null;
  private _emitsType: string | null = null;
  private _faultsType: string | null = null;

  constructor(name: string) {
    this.name = name;
    this.prefix = `/api/${name}`;
  }

  /** Add an endpoint to this service. */
  endpoint(method: string, path: string, handlerName?: string, impl?: HandlerImpl): this {
    if (!handlerName) {
      let slug = path.replace(/^\/+|\/+$/g, '').replace(/\//g, '_').replace(/:/g, '');
      if (!slug) slug = 'index';
      handlerName = `${method.toLowerCase()}_${slug}`;
    }
    this._endpoints.push({ method, path, handler: handlerName, impl: impl ?? stub() });
    return this;
  }

  /** Declare managed state type (semantic contract). */
  state(typeName: string): this {
    this._stateType = typeName;
    return this;
  }

  /** Declare emitted event type (semantic contract). */
  emits(typeName: string): this {
    this._emitsType = typeName;
    return this;
  }

  /** Declare fault type (semantic contract). */
  faults(typeName: string): this {
    this._faultsType = typeName;
    return this;
  }

  // Expose internals for YAML generation (read-only accessors)
  get endpoints(): EndpointEntry[] { return this._endpoints; }
  get stateType(): string | null { return this._stateType; }
  get emitsType(): string | null { return this._emitsType; }
  get faultsType(): string | null { return this._faultsType; }
}

// ============================================================================
// VilServer
// ============================================================================

interface ServerEndpointEntry {
  method: string;
  path: string;
  handler: string;
  exec_class?: string;
  input: SchemaEntry | null;
  output: SchemaEntry | null;
  upstream: UpstreamSpec | null;
}

interface MeshRoute {
  from: string;
  to: string;
  lane: string;
}

export class VilServer {
  name: string;
  port: number;
  private _services: ServiceProcess[] = [];
  private _endpoints: ServerEndpointEntry[] = [];
  private _semanticTypes: SemanticEntry[] = [];
  private _errors: ErrorEntry[] = [];
  private _state: StateEntry | null = null;
  private _mesh: { routes: MeshRoute[] } | null = null;
  private _failover: FailoverEntry[] = [];
  private _sseEvents: EventEntry[] = [];
  private _wsEvents: EventEntry[] = [];

  constructor(name: string, port = 8080) {
    this.name = name;
    this.port = port;
  }

  // -- HTTP method registration ---------------------------------------------

  /** Register a GET endpoint. */
  get(
    path: string,
    opts: {
      input?: Record<string, any>;
      output?: Record<string, any>;
      upstream?: UpstreamSpec;
      handler?: string;
      execClass?: string;
    } = {},
  ): this {
    this._addEndpoint('GET', path, opts.input, opts.output, opts.upstream, opts.handler, opts.execClass);
    return this;
  }

  /** Register a POST endpoint. */
  post(
    path: string,
    opts: {
      input?: Record<string, any>;
      output?: Record<string, any>;
      upstream?: UpstreamSpec;
      handler?: string;
      execClass?: string;
    } = {},
  ): this {
    this._addEndpoint('POST', path, opts.input, opts.output, opts.upstream, opts.handler, opts.execClass);
    return this;
  }

  /** Register a PUT endpoint. */
  put(
    path: string,
    opts: {
      input?: Record<string, any>;
      output?: Record<string, any>;
      upstream?: UpstreamSpec;
      handler?: string;
      execClass?: string;
    } = {},
  ): this {
    this._addEndpoint('PUT', path, opts.input, opts.output, opts.upstream, opts.handler, opts.execClass);
    return this;
  }

  /** Register a DELETE endpoint. */
  delete(
    path: string,
    opts: { handler?: string; execClass?: string } = {},
  ): this {
    this._addEndpoint('DELETE', path, undefined, undefined, undefined, opts.handler, opts.execClass);
    return this;
  }

  private _addEndpoint(
    method: string,
    path: string,
    input?: Record<string, any>,
    output?: Record<string, any>,
    upstream?: UpstreamSpec,
    handler?: string,
    execClass?: string,
  ): void {
    if (!handler) {
      let slug = path.replace(/^\/+|\/+$/g, '').replace(/\//g, '_').replace(/:/g, '');
      if (!slug) slug = 'index';
      handler = `${method.toLowerCase()}_${slug}`;
    }
    const ep: ServerEndpointEntry = {
      method,
      path,
      handler,
      input: buildSchema(input ?? null),
      output: buildSchema(output ?? null),
      upstream: upstream ?? null,
    };
    if (execClass) {
      ep.exec_class = execClass;
    }
    this._endpoints.push(ep);
  }

  // -- Semantic type declarations -------------------------------------------

  /** Declare a semantic type (state/event/fault/decision). */
  semanticType(
    name: string,
    kind: string,
    fields?: Record<string, string>,
    variants?: string[],
  ): this {
    this._semanticTypes.push(makeSemanticEntry(name, kind, fields, variants));
    return this;
  }

  /** Shorthand: declare a semantic state type AND set service state. */
  state(name: string, fields: Record<string, string>): this {
    this.semanticType(name, 'state', fields);
    this._state = {
      type: 'private_heap',
      fields: Object.entries(fields).map(([n, t]) => ({ name: n, type: t })),
    };
    return this;
  }

  /** Shorthand: declare a semantic event type. */
  event(name: string, fields: Record<string, string>): this {
    return this.semanticType(name, 'event', fields);
  }

  /** Shorthand: declare a semantic fault type. */
  fault(name: string, variants?: string[]): this {
    return this.semanticType(name, 'fault', undefined, variants || []);
  }

  /** Shorthand: declare a semantic decision type. */
  decision(name: string, fields: Record<string, string>): this {
    return this.semanticType(name, 'decision', fields);
  }

  /** Declare a VilError variant. */
  error(
    name: string,
    status: number,
    opts: { code?: string; retry?: boolean; fields?: Record<string, string> } = {},
  ): this {
    this._errors.push({
      name,
      status,
      code: opts.code,
      retry: opts.retry,
      fields: Object.entries(opts.fields || {}).map(([n, t]) => ({ name: n, type: t })),
    });
    return this;
  }

  // -- Mesh / Failover ------------------------------------------------------

  /** Declare Tri-Lane mesh routes. */
  mesh(routes: MeshRoute[]): this {
    this._mesh = { routes };
    return this;
  }

  /** Declare a failover entry. */
  failover(primary: string, backup: string, strategy = 'immediate'): this {
    this._failover.push({ primary, backup, strategy });
    return this;
  }

  // -- Event declarations ---------------------------------------------------

  /** Declare an SSE event type. */
  sseEvent(name: string, fields: Record<string, string>, topic?: string): this {
    this._sseEvents.push({
      name,
      topic,
      fields: Object.entries(fields).map(([n, t]) => ({ name: n, type: t })),
    });
    return this;
  }

  /** Declare a WebSocket event type. */
  wsEvent(name: string, topic?: string, fields?: Record<string, string>): this {
    this._wsEvents.push({
      name,
      topic,
      fields: Object.entries(fields || {}).map(([n, t]) => ({ name: n, type: t })),
    });
    return this;
  }

  // -- ServiceProcess composition -------------------------------------------

  /** Create and register a VX ServiceProcess. */
  service(nameOrSvc: string | ServiceProcess, prefix?: string): ServiceProcess {
    let svc: ServiceProcess;
    if (typeof nameOrSvc === 'string') {
      svc = new ServiceProcess(nameOrSvc);
    } else {
      svc = nameOrSvc;
    }
    if (prefix) {
      svc.prefix = prefix;
    }
    this._services.push(svc);
    return svc;
  }

  /** Enable observer mode. (Placeholder for future VIL observer feature.) */
  observer(enabled: boolean): this {
    // Reserved for observer mode — no YAML emission yet, mirrors Python SDK surface.
    return this;
  }

  // -- YAML generation ------------------------------------------------------

  /** Generate YAML manifest string for `vil compile`. */
  toYaml(): string {
    const lines: string[] = [];
    lines.push('vil_version: "6.0.0"');
    lines.push(`name: ${this.name}`);
    lines.push(`port: ${this.port}`);
    lines.push('token: shm');
    lines.push('mode: server');

    lines.push(...yamlSemanticTypes(this._semanticTypes));
    lines.push(...yamlErrors(this._errors));
    lines.push(...yamlState(this._state));

    // Mesh
    if (this._mesh) {
      lines.push('mesh:');
      lines.push('  routes:');
      for (const r of this._mesh.routes) {
        lines.push(`    - from: ${r.from}`);
        lines.push(`      to: ${r.to}`);
        lines.push(`      lane: ${r.lane}`);
      }
    }

    lines.push(...yamlFailover(this._failover));
    lines.push(...yamlEvents(this._sseEvents, 'sse_events'));
    lines.push(...yamlEvents(this._wsEvents, 'ws_events'));

    // Endpoints (server mode)
    if (this._endpoints.length > 0) {
      lines.push('endpoints:');
      for (const ep of this._endpoints) {
        lines.push(`  - method: ${ep.method}`);
        lines.push(`    path: "${ep.path}"`);
        lines.push(`    handler: ${ep.handler}`);
        if (ep.exec_class) {
          lines.push(`    exec_class: ${ep.exec_class}`);
        }
        if (ep.input) {
          lines.push('    input:');
          lines.push(`      type: ${ep.input.type}`);
          lines.push('      fields:');
          lines.push(...yamlFields(ep.input.fields, 8));
        }
        if (ep.output) {
          lines.push('    output:');
          lines.push(`      type: ${ep.output.type}`);
          lines.push('      fields:');
          lines.push(...yamlFields(ep.output.fields, 8));
        }
        if (ep.upstream) {
          const u = ep.upstream;
          lines.push('    upstream:');
          lines.push(`      type: ${u.type}`);
          lines.push(`      url: "${u.url}"`);
          if (u.method) {
            lines.push(`      method: ${u.method}`);
          }
          if (u.body_template) {
            lines.push(`      body_template: ${JSON.stringify(u.body_template)}`);
          }
        }
      }
    }

    // Services (VX app mode)
    if (this._services.length > 0) {
      lines.push('');
      lines.push('services:');
      for (const svc of this._services) {
        lines.push(`  - name: ${svc.name}`);
        lines.push(`    prefix: ${svc.prefix}`);
        if (svc.emitsType) {
          lines.push(`    emits: ${svc.emitsType}`);
        }
        if (svc.faultsType) {
          lines.push(`    faults: ${svc.faultsType}`);
        }
        if (svc.stateType) {
          lines.push(`    manages: ${svc.stateType}`);
        }
        if (svc.endpoints.length > 0) {
          lines.push('    endpoints:');
          for (const ep of svc.endpoints) {
            lines.push(`      - method: ${ep.method}`);
            lines.push(`        path: ${ep.path}`);
            lines.push(`        handler: ${ep.handler}`);
            lines.push(...yamlHandlerImpl(ep.impl, 8));
          }
        }
      }
    }

    return lines.join('\n') + '\n';
  }

  /**
   * Call `vil compile` with the generated YAML manifest.
   * If VIL_COMPILE_MODE=manifest, print YAML to stdout and exit.
   */
  compile(release = true): void {
    if (process.env.VIL_COMPILE_MODE === 'manifest') {
      process.stdout.write(this.toYaml());
      return;
    }

    const tmpDir = os.tmpdir();
    const manifestPath = path.join(tmpDir, `vil-${this.name}-${Date.now()}.yaml`);
    fs.writeFileSync(manifestPath, this.toYaml(), 'utf-8');

    const cmd = ['vil', 'compile', '--manifest', manifestPath];
    if (release) {
      cmd.push('--release');
    }
    cmd.push('--output', this.name);

    console.log(`  Compiling server: ${this.name}`);
    console.log(`  Command: ${cmd.join(' ')}`);
    try {
      execFileSync(cmd[0], cmd.slice(1), { stdio: 'inherit' });
    } catch (err: any) {
      if (err.code === 'ENOENT') {
        console.log("\n  'vil' CLI not found. Install with: cargo install vil_cli");
        console.log(`  Manifest written to: ${manifestPath}`);
      } else {
        console.log(`\n  Compilation failed (exit code ${err.status})`);
        console.log(`  Manifest: ${manifestPath}`);
      }
    }
  }
}
